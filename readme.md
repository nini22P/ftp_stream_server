# FTP Stream Server

This server streams files from an FTP server over HTTP.

## Endpoint

`GET /{filename}`

*   **`filename`:** The file to stream.
*   **Query Parameters:**
    *   `addr` (required): FTP server address.
    *   `port` (optional, default: 21): FTP server port.
    *   `user` (optional, default: anonymous): FTP username.
    *   `pass` (optional, default: ""): FTP password.

## Usage Examples

### With User/Password
```
http://<server_ip>:<port>/file.txt?addr=ftp.example.com&port=2121&user=test&pass=pass
```

### Anonymous Access

```
http://<server_ip>:<port>/file.txt?addr=ftp.example.com&port=2121
```

### Default Port and Anonymous

```
http://<server_ip>:<port>/file.txt?addr=ftp.example.com
```