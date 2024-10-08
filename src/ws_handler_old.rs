use std::sync::Arc;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc;
use log::error;
use sqlx::{pool::Pool, sqlite::Sqlite};
use tungstenite::Error::{ConnectionClosed, Io};
use tungstenite::{Error, Message};
use crate::error::{ConnectionClosedError, NoSuchKeyError};
use crate::{Client, ClientList, WsStream};
use crate::structs::respond::Respond;

pub async fn ws_handler(ws_stream: WsStream, sql_pool: Arc<Pool<Sqlite>>, mut client_list: ClientList) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let (mut write, mut read) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<String>(4);
    //状态码定义：0为初始化，1为连接成功，只有为0时才进行验证流程
    let mut status_code: i8 = 0;
    let mut client_key = String::new();
    // 初始化链接
    while status_code == 0 {
        if let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(msg)) =>  {
                        if status_code == 0 {
                            // 获取密钥
                            let key = msg.to_string();
                            // 验证密钥是否存在
                            let result: Result<Option<(String, )>, sqlx::Error> = sqlx::query_as("SELECT name FROM server_info WHERE key = ?")
                                .bind(&key)
                                .fetch_optional(&*sql_pool)
                                .await;
                            match result {
                                Ok(Some(row)) => {
                                    println!("客户端{}上线", row.0);
                                    status_code = 1;
                                    // 获取锁
                                    let mut client_list = client_list.lock().await;
                                    // 将客户端信息推送至客户端列表
                                    client_list.push(Client { client_key: key.clone(), client_handler: tx.clone() });
                                    client_key = key;
                                    // 释放锁
                                    drop(client_list);
                                    // 发送响应数据
                                    let respond = Respond { code: 0, msg: row.0 };
                                    let _ = write.send(Message::Binary(serde_json::to_vec(&respond).unwrap())).await;
                                    loop {
                                        let add_player_respond = Respond { code: 2, msg: rx.recv().await.unwrap() };
                                        let _ = write.send(Message::Binary(serde_json::to_vec(&add_player_respond).unwrap())).await;
                                        println!("1");
                                    }
                                }
                                Ok(None) => {
                                    eprintln!("客户端发送了无效的key，断开链接");
                                    let _ = write.send(Message::Close(None)).await;
                                    return Err(NoSuchKeyError.into());
                                }
                                Err(e) => {
                                    error!("Failed to execute query: {}", e);
                                    let _ = write.send(Message::Close(None)).await;
                                    return Err(ConnectionClosedError.into());
                                }
                            }
                        }
                    }
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                        // Ping and Pong messages can be handled here
                    }
                    Ok(Message::Close(_)) => {
                        println!("Client closed the connection");
                        if status_code == 1 {
                            remove_client(&mut client_list, client_key).await;
                        }
                        break;
                    }
                _ => {}
            }
        }
        while status_code == 1 {
            tokio::select! {
            Some(msg) = read.next() => {
                match msg {
                    Ok(Message::Close(_)) => {
                        println!("Client closed the connection");
                        remove_client(&mut client_list, client_key.clone()).await;
                        break;
                    }
                    Err(e) => match e {
                        ConnectionClosed => {
                            println!("客户端断开链接");
                            remove_client(&mut client_list, client_key.clone()).await;
                            break;
                        }
                        Io(ref err) if err.kind() == std::io::ErrorKind::ConnectionReset => {
                            println!("客户端异常退出");
                            remove_client(&mut client_list, client_key.clone()).await;
                            break;
                        }
                        _ => {
                            println!("Other error: {}", e);
                            remove_client(&mut client_list, client_key.clone()).await;
                            break;
                        }
                    },
                    _ => {}
                }
            },
            Some(msg) = rx.recv() => {
                let add_player_respond = Respond { code: 2, msg };
                let _ = write.send(Message::Binary(serde_json::to_vec(&add_player_respond).unwrap())).await;
                println!("1");
            }
        }
        }
    }
    Ok(())
}

async fn remove_client(client_list: &mut ClientList, client_key: String) {
    let mut client_list = client_list.lock().await;
    client_list.retain(|client| client.client_key != client_key);
}
