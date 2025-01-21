use axum::{
    body::Body,
    extract::{Path, Query},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
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
    headers: HeaderMap,
) -> Result<Response<Body>, (axum::http::StatusCode, String)> {
    println!("Requested file: {}", filename);

    let addr = query.addr.clone();
    let port = query.port.clone().unwrap_or(21);
    let user = query.user.clone().unwrap_or("anonymous".to_owned());
    let pass = query.pass.clone().unwrap_or("".to_owned());

    let headers = headers.clone();
    let default_range = axum::http::header::HeaderValue::from_static("bytes=0-");
    let range = headers.get("Range").unwrap_or(&default_range);

    let range_start: usize = range
        .to_str()
        .unwrap_or("")
        .split("-")
        .next()
        .unwrap_or("")
        .split("=")
        .last()
        .unwrap_or("")
        .parse()
        .unwrap_or(0);

    let range_end: usize = range
        .to_str()
        .unwrap_or("")
        .split("-")
        .last()
        .unwrap_or("")
        .parse()
        .unwrap_or(0);

    println!("Requested start: {}, end: {}", range_start, range_end);

    let addr = match addr {
        Some(addr) => addr,
        None => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Missing 'addr' parameter".to_string(),
            ));
        }
    };

    let ftp_addr = format!("{}:{}", addr, port);

    let mut ftp_stream = AsyncFtpStream::connect(ftp_addr).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Connection failed: {}", e),
        )
    })?;

    ftp_stream.login(user, pass).await.map_err(|e| {
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

    let file_size = ftp_stream.size(&filename).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            format!("File not found: {}", e),
        )
    })?;

    let transfer_size = if range_end > 0 && range_end < file_size && range_end > range_start {
        range_end - range_start
    } else {
        file_size - range_start
    };

    println!(
        "Transfer start: {}, Transfer size: {}",
        range_start, transfer_size,
    );

    ftp_stream.resume_transfer(range_start).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to resume transfer: {}", e),
        )
    })?;

    let data_stream = ftp_stream
        .retr_as_stream(filename.clone())
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::NOT_FOUND,
                format!("File not found: {}", e),
            )
        })?;

    let body = Body::from_stream(ReaderStream::new(data_stream.compat()));

    let mut headers = HeaderMap::new();
    headers.insert("Accept-Ranges", "bytes".parse().unwrap());
    headers.insert(
        "Content-Disposition",
        format!(
            "attachment; filename={}; filename*=UTF-8''{}",
            filename.clone(),
            filename
        )
        .parse()
        .unwrap(),
    );
    headers.insert(
        "Content-Length",
        (transfer_size).to_string().parse().unwrap(),
    );
    headers.insert(
        "Content-Range",
        format!(
            "bytes {}-{}/{}",
            range_start,
            if range_end > 0 && range_end < file_size && range_end > range_start {
                range_end
            } else {
                file_size - 1
            },
            file_size
        )
        .parse()
        .unwrap(),
    );
    headers.insert("Content-Type", "application/octet-stream".parse().unwrap());

    Ok((headers, body).into_response())
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/{*filename}", get(stream_ftp_file));
    // .layer(ServiceBuilder::new().layer(TimeoutLayer::new(Duration::from_secs(10))));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
