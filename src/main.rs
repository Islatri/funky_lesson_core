use funky_lesson_core::app::{enroll_courses, get_courses, login, print_courses, set_batch};
use funky_lesson_core::error::{ErrorKind, Result};
use funky_lesson_core::request::create_client;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 4 {
        println!(
            "用法: {} 用户名 密码 选课批次ID（从0开始） <有循环就填个数>",
            args[0]
        );
        return Ok(());
    }

    println!("args: {:?}", args);

    let username = args[1].clone();
    let password = args[2].clone();
    let batch_idx: usize = args[3]
        .parse()
        .map_err(|e| ErrorKind::ParseError(format!("Invalid batch index: {}", e)))?;
    let mut debug_request_count = 0;

    loop {
        let client = create_client().await?;

        let (token, batch_list) = loop {
            match login(&client, &username, &password).await {
                Ok(result) => break result,
                Err(e) => {
                    println!("登录失败: {}，重试中...", e);
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        };

        // 设置批次
        let batch_id = set_batch(&client, &token, &batch_list, batch_idx).await?;

        // 获取课程列表
        let (selected_courses, favorite_courses) = get_courses(&client, &token, &batch_id).await?;

        // 打印课程信息
        print_courses(&selected_courses, &favorite_courses);

        // 开始选课
        enroll_courses(&client, &token, &batch_id, &favorite_courses, true).await?;

        // 更新并打印已选课程
        let (selected_courses, _) = get_courses(&client, &token, &batch_id).await?;
        print_courses(&selected_courses, &[]);

        debug_request_count += 1;
        println!("DEBUG_REQUEST_COUNT: {}\n", debug_request_count);

        // 如果不是循环模式则退出
        if args.len() == 4 {
            break;
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    Ok(())
}
