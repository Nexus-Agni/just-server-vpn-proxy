# Postman Test Collection for PQC Proxy Server

## 🚀 Quick Setup

1. Start the server: `cargo run`
2. Server runs on: `http://localhost:8888`
3. Import these requests into Postman

## 📋 Test Requests

### 1. Main HTTP Proxy - GET Request
```
Method: GET
URL: http://localhost:8888/proxy?url=https://httpbin.org/get
Headers: None required
Body: None
```

### 2. Main HTTP Proxy - POST with JSON
```
Method: POST
URL: http://localhost:8888/proxy?url=https://httpbin.org/post
Headers: Content-Type: application/json
Body (raw JSON):
{
  "name": "Postman Test",
  "email": "test@example.com",
  "message": "Testing PQC Proxy Server",
  "timestamp": "2025-07-15T10:30:00Z"
}
```

### 3. Main HTTP Proxy - PUT Request
```
Method: PUT
URL: http://localhost:8888/proxy?url=https://httpbin.org/put
Headers: Content-Type: application/json
Body (raw JSON):
{
  "id": 12345,
  "action": "update",
  "data": {
    "field1": "updated_value",
    "field2": "another_value"
  }
}
```

### 4. Main HTTP Proxy - DELETE Request
```
Method: DELETE
URL: http://localhost:8888/proxy?url=https://httpbin.org/delete
Headers: Content-Type: application/json
Body (raw JSON):
{
  "id": 12345,
  "reason": "test_deletion"
}
```

### 5. Error Test - Missing URL Parameter
```
Method: GET
URL: http://localhost:8888/proxy
Expected: 400 Bad Request with error message
```

### 6. Error Test - Invalid URL
```
Method: GET
URL: http://localhost:8888/proxy?url=invalid-url-format
Expected: 500 Internal Server Error
```

### 7. Binary Content Test
```
Method: GET
URL: http://localhost:8888/proxy?url=https://httpbin.org/image/png
Expected: PNG image data (check Content-Type: image/png)
```

### 8. Status Code Forwarding Test
```
Method: GET
URL: http://localhost:8888/proxy?url=https://httpbin.org/status/404
Expected: 404 Not Found status
```

### 9. Custom Headers Test
```
Method: GET
URL: http://localhost:8888/proxy?url=https://httpbin.org/headers
Headers: 
  X-Custom-Header: test-value
  Authorization: Bearer test-token
Expected: Headers should appear in response
```

### 10. Legacy Proxy (PQC Enhanced)
```
Method: POST
URL: http://localhost:8888/proxy-legacy
Headers: Content-Type: application/json
Body (raw JSON):
{
  "url": "https://httpbin.org/json"
}
Expected: JSON response with PQC session data
```

### 11. PQC Information
```
Method: GET
URL: http://localhost:8888/pqc-info
Expected: JSON with PQC algorithms and public keys
```

### 12. PQC Proxy
```
Method: POST
URL: http://localhost:8888/pqc-proxy
Headers: Content-Type: application/json
Body (raw JSON):
{
  "url": "https://httpbin.org/html"
}
Expected: HTML content with PQC headers
```

### 13. PQC Handshake
```
Method: POST
URL: http://localhost:8888/pqc-handshake
Headers: Content-Type: application/json
Body (raw JSON):
{
  "kyber_pk": "test_key_data",
  "dilithium_pk": "test_key_data",
  "sphincs_pk": "test_key_data"
}
Expected: PQC session response (may fail with test keys)
```

## ✅ Expected Results

### Successful Proxy Response:
- Status: 200 OK (or forwarded status from target)
- Headers: CORS headers + forwarded response headers
- Body: Raw response from target URL

### Error Responses:
- 400 Bad Request: Missing URL parameter
- 500 Internal Server Error: Invalid URL or network issues

### PQC Enhanced Responses:
- Additional headers: X-PQC-Content-Hash, X-PQC-Content-Signature
- JSON structure with PQC session data and public keys

## 🔍 Validation Checklist

- [ ] Main proxy forwards all HTTP methods correctly
- [ ] Request body is preserved and forwarded
- [ ] Response status codes are preserved
- [ ] Custom headers are forwarded both ways
- [ ] Binary content (images) works correctly
- [ ] Error handling works for missing/invalid URLs
- [ ] CORS headers are present in all responses
- [ ] PQC endpoints return expected cryptographic data
- [ ] Legacy proxy maintains backward compatibility
