use axum::{body::Body, extract::Path, extract::Query, response::Response, routing::get, Router};
use std::net::SocketAddr;
use suppaftp::{types::FileType, AsyncFtpStream};
use tokio::net::TcpListener;
use tokio_util::{compat::FuturesAsyncReadCompatExt, io::ReaderStream};

#[derive(serde::Deserialize)]
struct MyQuery {
    addr: Option<String>,
    port: Option<u16>,
    user: Option<String>,
    pass: Option<String>,
}

async fn stream_ftp_file(
    Path(filename): Path<String>,
    Query(query): Query<MyQuery>,
) -> Result<Response<Body>, (axum::http::StatusCode, String)> {
    let addr = query.addr.clone();
    let port = query.port.clone();
    let user = query.user.clone();
    let pass = query.pass.clone();

    let addr = match addr {
        Some(addr) => addr,
        None => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Missing 'addr' parameter".to_string(),
            ));
        }
    };

    let ftp_addr = format!("{}:{}", addr, port.unwrap_or(21));

    let mut ftp_stream = AsyncFtpStream::connect(ftp_addr).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Connection failed: {}", e),
        )
    })?;

    ftp_stream
        .login(
            user.unwrap_or("anonymous".to_owned()),
            pass.unwrap_or("".to_owned()),
        )
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::UNAUTHORIZED,
                format!("Login failed: {}", e),
            )
        })?;

    ftp_stream
        .transfer_type(FileType::Binary)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to set transfer type: {}", e),
            )
        })?;

    let data_stream = ftp_stream.retr_as_stream(filename).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            format!("File not found: {}", e),
        )
    })?;

    let body = Body::from_stream(ReaderStream::new(data_stream.compat()));

    Ok(Response::new(body))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/stream/{*filename}", get(stream_ftp_file));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
