use server::auth::password;
use server::config::Config;
use server::db;
use uuid::Uuid;

struct SeedUser {
    username: &'static str,
    email: &'static str,
    display_name: &'static str,
    role: &'static str,
    password_env: &'static str,
    default_password: &'static str,
}

const SEED_USERS: &[SeedUser] = &[
    SeedUser {
        username: "admin",
        email: "admin@example.com",
        display_name: "Admin Account",
        role: "admin",
        password_env: "SEED_ADMIN_PASSWORD",
        default_password: "adminpass123",
    },
    SeedUser {
        username: "client",
        email: "client@example.com",
        display_name: "Client Account",
        role: "member",
        password_env: "SEED_CLIENT_PASSWORD",
        default_password: "clientpass123",
    },
];

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let config = Config::from_env().unwrap_or_else(|err| {
        eprintln!("failed to load configuration: {err}");
        std::process::exit(1);
    });

    let db = db::connect(&config.database_url)
        .await
        .unwrap_or_else(|err| {
            eprintln!("failed to connect to database: {err}");
            std::process::exit(1);
        });

    for user in SEED_USERS {
        let existing: Option<String> =
            sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
                .bind(user.username)
                .fetch_optional(&db)
                .await
                .unwrap_or_else(|err| {
                    eprintln!(
                        "failed to check for existing user `{}`: {err}",
                        user.username
                    );
                    std::process::exit(1);
                });

        if existing.is_some() {
            println!("skipped {} (`{}` already exists)", user.role, user.username);
            continue;
        }

        let password =
            std::env::var(user.password_env).unwrap_or_else(|_| user.default_password.to_string());
        let password_hash = password::hash_password(&password).unwrap_or_else(|err| {
            eprintln!("failed to hash password for `{}`: {err}", user.username);
            std::process::exit(1);
        });
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, display_name, role) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user.username)
        .bind(user.email)
        .bind(&password_hash)
        .bind(user.display_name)
        .bind(user.role)
        .execute(&db)
        .await
        .unwrap_or_else(|err| {
            eprintln!("failed to insert `{}`: {err}", user.username);
            std::process::exit(1);
        });

        println!(
            "seeded {} -> username: {}, password: {}",
            user.role, user.username, password
        );
    }
}
