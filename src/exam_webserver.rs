use actix_web::{App, HttpResponse, HttpServer, Responder, web};

pub async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello, Actix!")
}
pub fn new_actix_server(){
    let sys = actix_rt::System::new();
    sys.block_on(async {
        let server = HttpServer::new(|| {
            App::new()
                .route("/", web::get().to(index))
        })
            .bind("127.0.0.1:8081")
            .expect("HTTP服务无法绑定端口")
            .run();
        server.await.expect("HTTP服务运行失败");
    });
    sys.run().expect("HTTP服务运行失败");
}