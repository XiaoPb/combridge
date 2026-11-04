//! Bearer Token 认证中间件。
//!
//! 所有 `/mcp` 请求必须携带 `Authorization: Bearer <token>`。
//! token 每次启动随机生成并写入 endpoint 发现文件，进程重启即失效。

use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// 校验 Bearer Token；通过则放行，否则 401。
pub async fn bearer_auth(State(expected): State<String>, request: Request, next: Next) -> Response {
    let provided = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim);

    match provided {
        Some(token) if constant_time_eq(token.as_bytes(), expected.as_bytes()) => {
            next.run(request).await
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            "invalid or missing bearer token\n",
        )
            .into_response(),
    }
}

/// 常数时间比较，避免时序侧信道。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(constant_time_eq(b"", b""));
    }
}
