use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::LazyLock;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::sync::RwLock;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", parse_with = "split")]
enum Command {
    Start,
    AddMe,
    DeleteMe,
    CreateQueue,
    ShowQueue,
    Info,
    Help,
}

static students: LazyLock<RwLock<HashMap<ChatId, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
//TODO:  tokio::sync::RwLock drop(guard)
static queue: LazyLock<RwLock<Vec<String>>> = LazyLock::new(|| RwLock::new(Vec::new()));

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Запускаем бота...");
    //$env:TELOXIDE_TOKEN = "TOKEN"
    //$env:RUST_LOG="info"
    let bot = Bot::from_env();
    Command::repl(bot, action).await;
    /*
    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        if let Some(text) = msg.text() {
            bot.send_message(msg.chat.id, text).await?;
        }
        Ok(())
    })
    .await;
     */
}

async fn action(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    {
        let name_for_logger: String = msg.from.clone().unwrap().first_name;
        log::info!("Поступило сообщение от {}",name_for_logger);
    }
    match cmd {
        Command::Help | Command::Start => {
            bot.send_message(msg.chat.id, "Приветики, это бот для организации очереди учебной группы P3*17.\nОн работает рандомизированным способом: первичный вид очереди определяется рандомом. Далее студни могут обменяться местами(я этого не делал), если хотят подойти в начале/конце практики.\n Доступные команды:\n/addme - добавить вас в список студней\n/deleteme - удалить вас из списка студней\n/createqueue - создать общую очередь для всех ( затирает прошлую )\n/showqueue - показать общую очередь для всех\n/info - показать зарегистрированных пользователей ( в будущем метрику хз )\n/help - ну...блин)").await?;
        }
        Command::AddMe => {
            add_me(bot, msg).await?;
        }
        Command::DeleteMe => {
            delete_me(bot, msg).await?;
        }
        Command::CreateQueue => {
            create_queue(bot, msg).await?;
        }
        Command::ShowQueue => {
            show_queue(bot, msg).await?;
        }
        Command::Info => {
            info(bot, msg).await?;
        }
    }

    Ok(())
}

async fn add_me(bot: Bot, msg: Message) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, "Начинаю добавлять...")
        .await?;

    let user_cont = {
        let map = students.read().await;
        map.contains_key(&msg.chat.id)
    };

    if user_cont {
    } else {
        let mut map = students.write().await;
        let user = msg.from.unwrap();
        let first = user.first_name;
        let last = user.last_name.unwrap_or_default();
        let username = format!("{} {}", first, last);
        map.insert(msg.chat.id, username);
    }
    bot.send_message(msg.chat.id, "Done").await?;
    Ok(())
}

async fn delete_me(bot: Bot, msg: Message) -> ResponseResult<()> {
    let user_cont = {
        let map = students.read().await;
        map.contains_key(&msg.chat.id)
    };

    if user_cont {
        let mut map = students.write().await;
        map.remove(&msg.chat.id);
    }
    bot.send_message(msg.chat.id, "Done").await?;
    Ok(())
}

async fn info(bot: Bot, msg: Message) -> ResponseResult<()> {
    let a: String = {
        let map = students.read().await;
        let mut res: String = String::new();
        let mut cnt: u32 = 0;
        for name in map.values() {
            res.push_str(format!("№{} ", cnt).as_str());
            res.push_str(name);
            res.push_str("\n");
            cnt += 1;
        }
        res
    };
    if a.is_empty() {
        bot.send_message(msg.chat.id, "Нико не зарегестрирован")
            .await?;
    } else {
        bot.send_message(msg.chat.id, a).await?;
    }

    Ok(())
}

async fn create_queue(bot: Bot, msg: Message) -> ResponseResult<()> {
    {
        let mut q = queue.write().await;
        q.clear();
        let map = students.read().await;

        for name in map.values() {
            q.push(name.clone())
        }
        let mut rng = rand::rng();
        q.shuffle(&mut rng);
    }
    let v: Vec<(ChatId, String)> = {
        let map = students.read().await;
        let mut res = Vec::new();
        for (id, name) in map.iter() {
            res.push((id.clone(), name.clone()));
        }
        res
    };
    let user = msg.from.unwrap();
    let first = user.first_name;
    let last = user.last_name.unwrap_or_default();
    let victim = format!("{} {}", first, last);

    for (id, name) in &v {
        match bot
            .send_message(
                *id,
                format!("{}, очередь была перемешана студнем {}", name, victim),
            )
            .await
        {
            Err(err) => {
                log::warn!(
                    "Не удалось отправить уведомление для {} (ID: {}): {:?}",
                    name,
                    id,
                    err
                );
            }
            Ok(_) => {}
        };
    }

    bot.send_message(msg.chat.id, "Перемешана").await?;
    Ok(())
}
async fn show_queue(bot: Bot, msg: Message) -> ResponseResult<()> {
    let a: String = {
        let vec = queue.read().await;
        let mut res: String = String::new();
        if vec.is_empty() {
            res = "Очередь пуста".to_string();
        } else {
            for name in vec.iter() {
                res.push_str(format!("{}\n", name).as_str());
            }
        }
        res
    };

    bot.send_message(msg.chat.id, a).await?;
    Ok(())
}
