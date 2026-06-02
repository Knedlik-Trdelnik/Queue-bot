use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::fs;
use tokio::sync::RwLock;
use toml::Table;

#[derive(Serialize, Deserialize)]
struct User {
    chat_id: ChatId,
    name: String,
    username: String,
    is_admin: bool,
}
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

static STUDENTS: LazyLock<RwLock<HashMap<ChatId, User>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static ADMINS: LazyLock<RwLock<HashMap<ChatId, User>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
//TODO:  tokio::sync::RwLock drop(guard)
static QUEUE: LazyLock<RwLock<Vec<String>>> = LazyLock::new(|| RwLock::new(Vec::new()));
//TODO: избавиться от множественных клонирований. В queue хватит &str

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Запускаем бота...");
    //$env:TELOXIDE_TOKEN = "TOKEN"
    //$env:RUST_LOG="info"
    parse_and_init().await;
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

async fn parse_and_init() {
    log::info!("Начинаем читать файл конфигурации...");
    let config_name = "config.toml";
    let config = fs::read_to_string(config_name).await;

    match config {
        Ok(config) => {
            log::info!("{}", config);
            let mut adm = ADMINS.write().await;
            let res = config.parse::<Table>();

            match res {
                Ok(table) => {
                    let users = table["admins"].as_array().unwrap();
                    for user in users.iter() {
                        let admin = User  {
                            chat_id: ChatId(user["chat_id"].as_integer().unwrap()),
                            name: user["name"].as_str().unwrap().to_string(),
                            username: user["username"].as_str().unwrap().to_string(),
                            is_admin: user["is_admin"].as_bool().unwrap(),
                        };


                        adm.insert(admin.chat_id, admin);
                    }
                }
                Err(err) => {
                    log::warn!("Не удалось прочитать админов - {}", err);
                }
            }
        }
        Err(err) => {
            log::warn!("Не удалось прочитать админов - {}", err);
        }
    }
}

async fn action(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    {
        let name_for_logger: String = msg.from.clone().unwrap().first_name;
        log::info!(
            "Поступило сообщение от {}. Username:@{}. chatId: {}. Команда: {}",
            name_for_logger,
            msg.from
                .as_ref()
                .unwrap()
                .username
                .as_ref()
                .unwrap()
                .as_str(),
            msg.chat.id,
            msg.text().unwrap()
        );
    }
    match cmd {
        Command::Help | Command::Start => {
            bot.send_message(msg.chat.id, "Приветики, это бот для организации очереди учебной группы P3*17.\n\
            Он работает рандомизированным способом: первичный вид очереди определяется рандомом. Далее студни могут обменяться местами(я этого не делал), если хотят подойти в начале/конце практики.\n\
            Доступные команды:\n/addme - добавить вас в список студней\n\
            /deleteme - удалить вас из списка студней\n\
            /createqueue - [только для админов] создать общую очередь для всех ( затирает прошлую )\n\
            /showqueue - показать общую очередь для всех\n\
            /info - показать зарегистрированных пользователей ( в будущем метрику хз )\n\
            /help - ну...блин)").await?;
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
        let map = STUDENTS.read().await;
        map.contains_key(&msg.chat.id)
    };
    if !user_cont {
        let mut map = STUDENTS.write().await;
        let user = msg.from.as_ref().unwrap();
        let first = user.first_name.clone();
        let last = user.last_name.clone().unwrap_or_default().clone();
        let username = format!("{} {}", first, last);
        let student = User {
            chat_id: msg.chat.id,
            name: username.clone(),
            username: msg
                .from
                .as_ref()
                .unwrap()
                .username
                .as_ref()
                .unwrap()
                .clone(),
            is_admin: false,
        };
        map.insert(msg.chat.id, student);
        let mut q = QUEUE.write().await;
        q.push(username);
    } else {
        bot.send_message(
            msg.chat.id,
            "Погоди, ты уже зарегистрирован. Зачем тебе все это.........???",
        )
        .await?;
    }
    bot.send_message(msg.chat.id, "Done (добавлен в конец очереди)")
        .await?;
    Ok(())
}

async fn delete_me(bot: Bot, msg: Message) -> ResponseResult<()> {
    let (user_cont, user_fl_name) = {
        let map = STUDENTS.read().await;
        let is_user_cont = map.contains_key(&msg.chat.id);
        let cht_id: ChatId;
        let mut user_fl_name_local: String = "".to_string();
        if is_user_cont {
            user_fl_name_local = map[&msg.chat.id].name.clone();
        }
        //(map.contains_key(&msg.chat.id), map[&msg.chat.id].clone())
        (is_user_cont, user_fl_name_local)
    };

    if user_cont {
        let mut map = STUDENTS.write().await;
        let mut q = QUEUE.write().await;

        let index = {
            let mut some_inx: usize = 0; //МММ как умом вот я умный ммм умом да очень умно ммм
            for u in q.iter() {
                if user_fl_name.eq(u) {
                    break;
                }
                some_inx += 1;
            }
            some_inx
        };
        q.remove(index);

        map.remove(&msg.chat.id);
    }
    bot.send_message(
        msg.chat.id,
        "Done (Удален из текущей очереди иииии...зарегистрированных пльзвтлй)",
    )
    .await?;
    Ok(())
}

async fn info(bot: Bot, msg: Message) -> ResponseResult<()> {
    let a: String = {
        let map = ADMINS.read().await;
        let mut res: String = String::new();
        let mut cnt: u32 = 1;
        res.push_str("Админы с правом перемешивать очередь\n");
        for user in map.values() {
            res.push_str(format!("№{} ", cnt).as_str());
            res.push_str(user.name.as_str());
            res.push_str(" || @");
            res.push_str(user.username.as_str());
            res.push_str("\n");
            cnt += 1;
        }
        let map = STUDENTS.read().await;
        res.push_str("Список зарегистрированных пльзвтлй\n");
        cnt = 1;
        for user in map.values() {
            res.push_str(format!("№{} ", cnt).as_str());
            res.push_str(user.name.as_str());
            res.push_str(" || @");
            res.push_str(user.username.as_str());
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
        let mut map = ADMINS.read().await;
        if !map.contains_key(&msg.chat.id) {
            bot.send_message(msg.chat.id, "Эй, ты не админ...фу...")
                .await?;
            return Ok(());
        }
    }

    {
        let mut q = QUEUE.write().await;
        q.clear();
        let map = STUDENTS.read().await;

        for user in map.values() {
            q.push(user.name.clone());
        }
        let mut rng = rand::rng();
        q.shuffle(&mut rng);
    }
    let v: Vec<(ChatId, String)> = {
        let map = STUDENTS.read().await;
        let mut res = Vec::new();
        for (id, user) in map.iter() {
            res.push((id.clone(), user.name.clone()));
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
        let vec = QUEUE.read().await;
        let mut res: String = String::new();
        if vec.is_empty() {
            res = "Очередь пуста".to_string();
        } else {
            let mut cnt: u32 = 1;
            for name in vec.iter() {
                res.push_str(format!("[{}] --> {}\n", cnt, name).as_str());
                cnt += 1;
            }
        }
        res
    };

    bot.send_message(msg.chat.id, a).await?;
    Ok(())
}
