////////////////////////////////////////////////////////////////////////////////
// This file is part of "Ad Astra", an embeddable scripting programming       //
// language platform.                                                         //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/ad-astra/blob/master/EULA.md               //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{
    io::{stdin, stdout, BufReader, Write},
    net::{TcpListener, TcpStream},
    process::exit,
    thread::{park_timeout, Builder, JoinHandle},
    time::Duration,
};

use log::{error, info, warn};

use crate::{
    runtime::PackageMeta,
    server::{
        logger::{LSP_CLIENT_LOG, LSP_SERVER_LOG},
        HealthCheck,
        LspLoggerConfig,
        LspServer,
        LspServerConfig,
        LspTransportConfig,
        RpcMessage,
        RpcReceiver,
        RpcSender,
    },
};

impl LspServer {
    /// Starts up the Language Server by establishing communication with the
    /// client via the configured communication channel.
    ///
    /// The lifecycle of the server is fully controlled by this function. Once
    /// the function returns, you should terminate the program's process.
    ///
    /// Note that this function spawns separate threads for handling incoming
    /// and outgoing RPC messages, even if the `multi_thread` flag in
    /// [LspServerConfig] is set to false. Therefore, this function is not
    /// suitable for wasm builds that require single-thread execution.
    ///
    /// If you need to manually control the server's lifecycle, including its
    /// communication channels, consider manually instantiating the server using
    /// the [LspServer::new] constructor instead.
    ///
    /// The `server_config` parameter specifies the general server configuration
    /// options.
    ///
    /// The `logger_config` parameter specifies the client and server
    /// logger configurations.
    ///
    /// The `transport_config` specifies the type of transport for RPC
    /// incoming and outgoing messages.
    ///
    /// The `package` parameter is the metadata of the
    /// [Script Package](crate::runtime::ScriptPackage) under which the server
    /// will analyze the client's source code files.
    pub fn startup(
        server_config: LspServerConfig,
        logger_config: LspLoggerConfig,
        transport_config: LspTransportConfig,
        package: &'static PackageMeta,
    ) {
        let (outgoing_sender, outgoing_receiver) = RpcMessage::channel();
        let (incoming_sender, incoming_receiver) = RpcMessage::channel();

        let server = Self::new(server_config, package, outgoing_sender);

        let logger = server.setup_logger(logger_config);

        info!(target: LSP_SERVER_LOG, " ----- Ad Astra LSP Server ----- ");
        info!(target: LSP_CLIENT_LOG, "Package: {package:#}.");

        match logger {
            true => info!(target: LSP_SERVER_LOG, "LSP Logger enabled."),
            false => warn!(target: LSP_SERVER_LOG, "LSP Logger was not set."),
        }

        let (incoming_thread, outgoing_thread) = match transport_config {
            LspTransportConfig::Stdio => {
                Self::setup_stdio_transport(incoming_sender, outgoing_receiver)
            }

            LspTransportConfig::TcpClient(addr) => {
                let stream = match TcpStream::connect(addr) {
                    Ok(stream) => stream,
                    Err(error) => panic!("Cannot connect to {addr}: {error}"),
                };

                info!(target: LSP_SERVER_LOG, "Connected to {addr}.");

                Self::setup_socket_transport(stream, incoming_sender, outgoing_receiver)
            }

            LspTransportConfig::TcpServer(addr) => {
                let server = match TcpListener::bind(addr) {
                    Ok(listener) => listener,
                    Err(error) => panic!("Binding to {addr} failure: {error}"),
                };

                info!(target: LSP_SERVER_LOG, "Server started at {addr}.");

                let stream = match server.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) => panic!("Remote connection to {addr} failure: {error}"),
                };

                info!(target: LSP_SERVER_LOG, "Remote connection established to {addr}.");

                Self::setup_socket_transport(stream, incoming_sender, outgoing_receiver)
            }
        };

        let health_check_thread = match server.health_check() {
            Some(health_check) => Some(Self::spawn_health_check_thread(health_check.clone())),
            None => None,
        };

        let main_loop_thread = server.spawn_main_loop_thread(incoming_receiver);

        if main_loop_thread.join().is_err() {
            panic!("LSP main loop thread panic.");
        }

        if incoming_thread.join().is_err() {
            panic!("LSP incoming thread panic.");
        }

        if outgoing_thread.join().is_err() {
            panic!("LSP outgoing thread panic.");
        }

        if let Some(health_check_thread) = health_check_thread {
            if health_check_thread.join().is_err() {
                panic!("LSP health check thread panic.");
            }
        }

        info!(target: LSP_SERVER_LOG, "Server finished.");
    }

    fn spawn_health_check_thread(health_check: HealthCheck) -> JoinHandle<()> {
        let thread = Builder::new()
            .name(String::from("Health check"))
            .spawn(move || {
                let health_check = health_check;

                info!(target: LSP_SERVER_LOG, "Health check thread started.");

                loop {
                    let timeout = health_check.timeout();

                    if timeout.is_zero() {
                        break;
                    }

                    let half = timeout / 2;

                    health_check.ping();

                    park_timeout(half);

                    for (name, pong) in health_check.check(true) {
                        warn!(target: LSP_SERVER_LOG, "{name} not responding in {:?}.", pong.elapsed());
                    }

                    park_timeout(half);
                }

                info!(target: LSP_SERVER_LOG, "Health check thread finished.");
            });

        match thread {
            Ok(handle) => handle,
            Err(error) => panic!("Failed to spawn health check thread. {error}",),
        }
    }

    fn spawn_main_loop_thread(self, incoming_receiver: RpcReceiver) -> JoinHandle<()> {
        let thread = Builder::new().name(String::from("Stdin")).spawn(move || {
            let mut server = self;
            let incoming_receiver = incoming_receiver;

            info!(target: LSP_SERVER_LOG, "Main loop thread started.");

            loop {
                let Ok(message) = incoming_receiver.recv() else {
                    break;
                };

                if message.is_exit() {
                    match server.shutting_down() {
                        true => {
                            info!(target: LSP_SERVER_LOG, "Normal exit.");
                            break;
                        }
                        false => {
                            error!(target: LSP_SERVER_LOG, "Abnormal exit.");
                            exit(1);
                        }
                    }
                }

                server.handle(message);
            }

            if let Some(health_check) = server.health_check() {
                health_check.set_timeout(Duration::ZERO);
            }

            drop(server);

            info!(target: LSP_SERVER_LOG, "Main loop thread finished.");
        });

        match thread {
            Ok(handle) => handle,
            Err(error) => panic!("Failed to spawn main loop thread. {error}",),
        }
    }

    fn setup_stdio_transport(
        incoming_sender: RpcSender,
        outgoing_receiver: RpcReceiver,
    ) -> (JoinHandle<()>, JoinHandle<()>) {
        let incoming_thread = Builder::new().name(String::from("Stdin")).spawn(move || {
            let mut stdin = stdin().lock();

            info!(target: LSP_SERVER_LOG, "Stdin thread started.");

            let incoming_sender = incoming_sender;

            loop {
                let Some(message) = RpcMessage::read(&mut stdin) else {
                    break;
                };

                if incoming_sender.send(message).is_err() {
                    break;
                }
            }

            info!(target: LSP_SERVER_LOG, "Stdin thread finished.");
        });

        let incoming_thread = match incoming_thread {
            Ok(handle) => handle,
            Err(error) => panic!("Failed to spawn Stdin thread. {error}",),
        };

        let outgoing_thread = Builder::new().name(String::from("Stdout")).spawn(move || {
            let mut stdout = stdout().lock();

            info!(target: LSP_SERVER_LOG, "Stdout thread started.");

            let outgoing_receiver = outgoing_receiver;

            loop {
                let Ok(message) = outgoing_receiver.recv() else {
                    break;
                };

                if !message.write(&mut stdout) {
                    break;
                }

                if let Err(error) = stdout.flush() {
                    error!(target: LSP_SERVER_LOG, "Stdout flush error: {error}");
                    break;
                }
            }

            info!(target: LSP_SERVER_LOG, "Stdout thread finished.");
        });

        let outgoing_thread = match outgoing_thread {
            Ok(handle) => handle,
            Err(error) => panic!("Failed to spawn Stdout thread. {error}",),
        };

        (incoming_thread, outgoing_thread)
    }

    fn setup_socket_transport(
        stream: TcpStream,
        incoming_sender: RpcSender,
        outgoing_receiver: RpcReceiver,
    ) -> (JoinHandle<()>, JoinHandle<()>) {
        let input_stream = match stream.try_clone() {
            Ok(stream) => stream,
            Err(error) => panic!("Failed to clone TCP stream: {error}",),
        };

        let incoming_thread = Builder::new()
            .name(String::from("TCP input"))
            .spawn(move || {
                let mut input_stream = BufReader::new(input_stream);

                info!(target: LSP_SERVER_LOG, "TCP input thread started.");

                let incoming_sender = incoming_sender;

                loop {
                    let Some(message) = RpcMessage::read(&mut input_stream) else {
                        break;
                    };

                    let exit = message.is_exit();

                    if incoming_sender.send(message).is_err() {
                        break;
                    }

                    if exit {
                        break;
                    }
                }

                info!(target: LSP_SERVER_LOG, "TCP input thread finished.");
            });

        let incoming_thread = match incoming_thread {
            Ok(handle) => handle,
            Err(error) => panic!("Failed to spawn TCP input thread. {error}",),
        };

        let output_stream = stream;

        let outgoing_thread = Builder::new()
            .name(String::from("TCP output"))
            .spawn(move || {
                let mut output_stream = output_stream;

                info!(target: LSP_SERVER_LOG, "TCP output thread started.");

                let outgoing_receiver = outgoing_receiver;

                loop {
                    let Ok(message) = outgoing_receiver.recv() else {
                        break;
                    };

                    if !message.write(&mut output_stream) {
                        break;
                    }

                    if let Err(error) = output_stream.flush() {
                        error!(
                            target: LSP_SERVER_LOG,
                            "TCP output stream flush error: {error}",
                        );
                        break;
                    }
                }

                info!(target: LSP_SERVER_LOG, "TCP output thread finished.");
            });

        let outgoing_thread = match outgoing_thread {
            Ok(handle) => handle,
            Err(error) => panic!("Failed to spawn TCP output thread. {error}",),
        };

        (incoming_thread, outgoing_thread)
    }
}
