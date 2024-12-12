use actix_web::{web, HttpResponse};
use serde_json::json;
use crate::email_server::EmailServerHandle;
use crate::r#struct::submit::{CaptchaResponse, RegisterRequest};

pub async fn register_pending(req_body: web::Json<RegisterRequest>, email_server: web::Data<EmailServerHandle>) -> HttpResponse{
    let email = &req_body.email;
    let server_name = &req_body.server_name;
    let captcha_token = &req_body.captcha_token;

    let params = [("secret", ""), ("response", captcha_token)];
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
            log::error!("Failed to send email: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"code": 500}))
        }
    }

}

// 验证token是否有效
pub async fn verify(path: web::Path<String>, email_server: web::Data<EmailServerHandle>) -> HttpResponse {
    // 从get请求中获取token参数
    let token =  path.into_inner();

    // 调用email_server进行验证
    let result = email_server.validate_token(token.to_string()).await;
    match result {
        Ok(_) => HttpResponse::Ok().json(json!({"code": 200})),
        Err(_) => HttpResponse::Unauthorized().json(json!({"code": 401})),
    }
}