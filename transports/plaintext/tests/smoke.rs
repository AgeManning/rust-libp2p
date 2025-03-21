// Copyright 2019 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use futures::io::{AsyncReadExt, AsyncWriteExt};
use libp2p_core::InboundUpgrade;
use libp2p_identity as identity;
use libp2p_plaintext::PlainText2Config;
use log::debug;
use quickcheck::QuickCheck;

#[test]
fn variable_msg_length() {
    let _ = env_logger::try_init();

    fn prop(msg: Vec<u8>) {
        let msg_to_send = msg.clone();
        let msg_to_receive = msg;

        let server_id = identity::Keypair::generate_ed25519();
        let server_id_public = server_id.public();

        let client_id = identity::Keypair::generate_ed25519();
        let client_id_public = client_id.public();

        let (server, client) = futures_ringbuf::Endpoint::pair(100, 100);

        futures::executor::block_on(async {
            let (
                (received_client_id, mut server_channel),
                (received_server_id, mut client_channel),
            ) = futures::future::try_join(
                PlainText2Config {
                    local_public_key: server_id_public,
                }
                .upgrade_inbound(server, ""),
                PlainText2Config {
                    local_public_key: client_id_public,
                }
                .upgrade_inbound(client, ""),
            )
            .await
            .unwrap();

            assert_eq!(received_server_id, server_id.public().to_peer_id());
            assert_eq!(received_client_id, client_id.public().to_peer_id());

            let client_fut = async {
                debug!("Client: writing message.");
                client_channel
                    .write_all(&msg_to_send)
                    .await
                    .expect("no error");
                debug!("Client: flushing channel.");
                client_channel.flush().await.expect("no error");
            };

            let server_fut = async {
                let mut server_buffer = vec![0; msg_to_receive.len()];
                debug!("Server: reading message.");
                server_channel
                    .read_exact(&mut server_buffer)
                    .await
                    .expect("reading client message");

                assert_eq!(server_buffer, msg_to_receive);
            };

            futures::future::join(server_fut, client_fut).await;
        })
    }

    QuickCheck::new()
        .max_tests(30)
        .quickcheck(prop as fn(Vec<u8>))
}
