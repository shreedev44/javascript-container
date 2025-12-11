use bincode::{Decode, Encode, config};
use std::process::Stdio;
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    process::Command,
};
use uuid::Uuid;

#[derive(Encode, Decode, Debug)]
enum MessageType {
    Execution,
    Stdin,
}

#[derive(Encode, Decode, Debug)]
struct Message {
    message_type: MessageType,
    language: String,
    code: String,
}

#[derive(thiserror::Error, Debug)]
enum HandlerError {
    #[error("Failed to listen on port: {port}")]
    ListenerError { port: u16 },

    #[error("File system error: {0}")]
    IoError(#[from] tokio::io::Error),

    #[error("Failed to decode: {0}")]
    DecodeError(#[from] bincode::error::DecodeError),

    #[error("Failed to encode: {0}")]
    EncodeError(#[from] bincode::error::EncodeError),

    #[error("Failed to spawn task")]
    SpawnError(#[from] tokio::task::JoinError),

    #[error("Failed to connect to child I/O")]
    ProcessIOError,
}

#[tokio::main]
async fn main() {
    listen_to_port(8000).await.expect("Failed");
}

async fn listen_to_port(port: u16) -> Result<(), HandlerError> {
    let address = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(address)
        .await
        .map_err(|_| HandlerError::ListenerError { port })?;

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::spawn(async move {
            handle_request(stream).await.expect("Failed to handle request");
        });
    }
}

async fn handle_request(mut stream: TcpStream) -> Result<(), HandlerError> {

    let message = read_content_from_stream(&mut stream).await?;
    handle_message(message, stream)
        .await?;

    Ok(())
}

async fn handle_message(message: Message, mut stream: TcpStream) -> Result<(), HandlerError> {
    let uuid = Uuid::new_v4();
    let dir_path = format!("temp/{}", uuid.to_string());

    fs::create_dir(&dir_path).await?;
    let path = format!("{}/script.js", &dir_path);

    fs::write(&path, message.code.as_bytes()).await?;

    let mut child = Command::new("node")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| HandlerError::ProcessIOError)?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| HandlerError::ProcessIOError)?;

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut stdout_open = true;
    let mut stderr_open = true;

    loop {
        tokio::select! {
            stdout_line_result = stdout_reader.next_line() => {
                let stdout_line = stdout_line_result?;
                match stdout_line {
                    Some(line) => {
                        let output = format!("{line}\n");
                        stream.write(output.as_bytes()).await?;
                        stream.flush().await?;
                    }
                    None => {
                        stdout_open = false;
                    }
                }
            }
            stderr_line_result = stderr_reader.next_line() => {
                let stderr_line = stderr_line_result?;
                match stderr_line {
                    Some(line) => {
                        let output = format!("{line}\n");
                        stream.write(output.as_bytes()).await?;
                        stream.flush().await?;
                    }
                    None => {
                        stderr_open = false;
                    }
                }
            }
            status = child.wait(), if !stdout_open && !stderr_open => {
                let exit_status = status?;
                println!("Child process exited with final status: {}", exit_status);
                break;
            }
        }
    }
    fs::remove_dir_all(dir_path).await?;

    Ok(())
}

async fn read_content_from_stream(stream: &mut TcpStream) -> Result<Message, HandlerError> {
    let mut content_length_buffer = [0u8; 4];
    stream.read_exact(&mut content_length_buffer).await?;

    let content_length = u32::from_ne_bytes(content_length_buffer);

    let mut message_buffer = vec![0u8; content_length as usize];
    stream.read_exact(&mut message_buffer).await?;

    let (message, _message_length): (Message, usize) =
        bincode::decode_from_slice(&message_buffer[..], config::standard())?;
    Ok(message)
}
