use actix_web::{web, HttpResponse};
use serde_json::json;
use crate::email_server::EmailServerHandle;
use crate::r#struct::submit::{CaptchaResponse, RegisterRequest};
use crate::sql_server::SqlServerHandle;

pub async fn register_pending(req_body: web::Json<RegisterRequest>, email_server: web::Data<EmailServerHandle>) -> HttpResponse{
    let email = &req_body.email;
    let server_name = &req_body.server_name;
    let captcha_token = &req_body.captcha_token;

    let params = [("secret", "ES_a48a8cc5a4594f80a3393ba309bb5654"), ("response", captcha_token)];
    let client = reqwest::Client::new();

    // 发送captcha验证请求
    let res = client.post("https://hcaptcha.com/siteverify")
        .form(&params)
        .send()
        .await;

    // 验证请求是否有效
    match res {
        Ok(response) => {
            if response.status().is_success() {
                let captcha_response: CaptchaResponse = response.json().await.unwrap();
                if captcha_response.success {
                    // 请求成功，继续下一步
                } else {
                    match captcha_response.error_codes {
                        Some(codes) => {
                            for code in codes {
                                log::error!("Captcha error code: {}", code);
                            }
                            return HttpResponse::InternalServerError().json(json!({"msg": "验证失败，请重试"}))
                        }
                        None => {
                            log::error!("Captcha error: No error codes returned.");
                            return HttpResponse::InternalServerError().json(json!({"msg": "验证失败，请重试"}))
                        }
                    }

                }
            }
        }
        Err(e) => {
            log::error!("Error sending captcha verify request: {}", e);
            return HttpResponse::InternalServerError().json(json!({"msg": "无法进行验证，请稍后重试"}))
        }
    }

    // 向email_server注册信息
    let register_token = email_server.send_token(email.to_string(), server_name.to_string()).await;
    match register_token {
        Ok(_) => HttpResponse::Ok().json(json!({"code": 200})),
        Err(e) => {
            log::error!("发送邮件时出错: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"code": 500}))
        }
    }

}

// 验证token是否有效
pub async fn verify(path: web::Path<String>, email_server: web::Data<EmailServerHandle>, sql_server_handle: web::Data<SqlServerHandle>) -> HttpResponse {
    // 从get请求中获取token参数
    let token =  path.into_inner();

    // 调用email_server进行验证
    let result = email_server.validate_token(token.to_string()).await;
    match result {
        Ok(server_name) => {
            match sql_server_handle.register_new_client(server_name).await{
                Ok(key) => HttpResponse::Ok().body(format!("注册成功，您的client_key为{},请谨慎保管",key)),
                Err(e) => {
                    log::error!("添加客户端信息时出错:{:?}",e);
                    HttpResponse::InternalServerError().body("服务器内部错误，请联系开发者处理")
                }
            }
        }
        Err(_) => HttpResponse::Unauthorized().body("链接错误或已过期，请再次检查后重试".to_string())
    }
}