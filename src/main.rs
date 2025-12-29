use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use console::{style, Emoji};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use dotenvy::dotenv;
use indicatif::{ProgressBar, ProgressStyle};
use lettre::{
    message::{header::ContentType, Attachment, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{env, fs, time::Duration};

static ROCKET: Emoji<'_, '_> = Emoji("🚀", "");
static MAIL: Emoji<'_, '_> = Emoji("📧", "");
static CHECK: Emoji<'_, '_> = Emoji("✅", "");
static CROSS: Emoji<'_, '_> = Emoji("❌", "");
static CLOCK: Emoji<'_, '_> = Emoji("⏰", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨", "");

const CONFIG_FILE: &str = "config.json";
const CV_FILE: &str = "cv.pdf";
const LOG_FILE: &str = "sent_log.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub profile: Profile,
    pub smtp: SmtpConfig,
    pub template: EmailTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub title: String,
    pub summary: String,
    pub skills: Vec<String>,
    pub experience_years: u8,
    pub linkedin: Option<String>,
    pub github: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
}

fn get_smtp_creds() -> Result<Credentials> {
    let user = env::var("SMTP_USER").context("SMTP_USER not set in .env")?;
    let pass = env::var("SMTP_PASS").context("SMTP_PASS not set in .env")?;
    Ok(Credentials::new(user, pass))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplate {
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentRecord {
    pub email: String,
    pub sent_at: DateTime<Local>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SentLog {
    pub records: Vec<SentRecord>,
}

fn load_config() -> Result<Config> {
    let content = fs::read_to_string(CONFIG_FILE).context("config.json not found")?;
    serde_json::from_str(&content).context("Invalid config.json")
}

fn load_cv() -> Result<Vec<u8>> {
    fs::read(CV_FILE).context("cv.pdf not found")
}

fn load_log() -> SentLog {
    fs::read_to_string(LOG_FILE)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

fn save_log(log: &SentLog) -> Result<()> {
    fs::write(LOG_FILE, serde_json::to_string_pretty(log)?)?;
    Ok(())
}

fn build_email(config: &Config) -> (String, String) {
    let p = &config.profile;
    let t = &config.template;
    
    let subj = t.subject
        .replace("{{name}}", &p.name)
        .replace("{{title}}", &p.title);
    
    let body = t.body
        .replace("{{name}}", &p.name)
        .replace("{{email}}", &p.email)
        .replace("{{phone}}", &p.phone)
        .replace("{{title}}", &p.title)
        .replace("{{summary}}", &p.summary)
        .replace("{{skills}}", &p.skills.join(", "))
        .replace("{{experience_years}}", &p.experience_years.to_string())
        .replace("{{linkedin}}", p.linkedin.as_deref().unwrap_or("N/A"))
        .replace("{{github}}", p.github.as_deref().unwrap_or("N/A"));
    
    (subj, body)
}

async fn send_email(config: &Config, to: &str, cv: &[u8]) -> Result<()> {
    let (subj, body) = build_email(config);
    
    let attach = Attachment::new("CV.pdf".into())
        .body(cv.to_vec(), ContentType::parse("application/pdf").unwrap());
    
    let msg = Message::builder()
        .from(config.profile.email.parse()?)
        .to(to.parse()?)
        .subject(subj)
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::plain(body))
                .singlepart(attach),
        )?;
    
    let creds = get_smtp_creds()?;
    
    let mailer: AsyncSmtpTransport<Tokio1Executor> = 
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp.host)?
            .port(config.smtp.port)
            .credentials(creds)
            .build();
    
    mailer.send(msg).await?;
    Ok(())
}

fn print_banner() {
    println!();
    println!("{}", style("╔═══════════════════════════════════════╗").cyan());
    println!("{}", style("║      JOB MAILER CLI                   ║").cyan());
    println!("{}", style("║      Envia candidaturas fácil!        ║").cyan());
    println!("{}", style("╚═══════════════════════════════════════╝").cyan());
    println!();
}

fn print_stats(log: &SentLog) {
    let total = log.records.len();
    let success = log.records.iter().filter(|r| r.success).count();
    let failed = total - success;
    
    println!();
    println!("{} {}", SPARKLE, style("Estatísticas").bold().yellow());
    println!("   Total enviados: {}", style(total).cyan());
    println!("   {} Sucesso: {}", CHECK, style(success).green());
    println!("   {} Falhados: {}", CROSS, style(failed).red());
    println!();
}

async fn send_single(config: &Config, cv: &[u8], log: &mut SentLog) -> Result<()> {
    let email: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("{} Email do destinatário", MAIL))
        .interact_text()?;
    
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")?,
    );
    spinner.set_message(format!("Enviando para {}...", style(&email).yellow()));
    spinner.enable_steady_tick(Duration::from_millis(80));
    
    let result = send_email(config, &email, cv).await;
    spinner.finish_and_clear();
    
    let record = SentRecord {
        email: email.clone(),
        sent_at: Local::now(),
        success: result.is_ok(),
        error: result.as_ref().err().map(|e| e.to_string()),
    };
    log.records.push(record);
    save_log(log)?;
    
    match result {
        Ok(_) => println!("{} Enviado para {}", CHECK, style(&email).green()),
        Err(e) => println!("{} Falhou {}: {}", CROSS, style(&email).red(), e),
    }
    
    Ok(())
}

async fn send_bulk(config: &Config, cv: &[u8], log: &mut SentLog) -> Result<()> {
    println!("{} Insere os emails (um por linha, linha vazia para terminar):", MAIL);
    
    let mut emails: Vec<String> = vec![];
    loop {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("  [{}]", emails.len() + 1))
            .allow_empty(true)
            .interact_text()?;
        
        if input.is_empty() { break; }
        if input.contains('@') {
            emails.push(input);
        } else {
            println!("   {} Email inválido, ignorado", CROSS);
        }
    }
    
    if emails.is_empty() {
        println!("{} Nenhum email inserido!", CROSS);
        return Ok(());
    }
    
    let min_delay: u64 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("{} Delay mínimo entre envios (segundos)", CLOCK))
        .default(30)
        .interact_text()?;
    
    let max_delay: u64 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("{} Delay máximo entre envios (segundos)", CLOCK))
        .default(60)
        .interact_text()?;
    
    println!();
    println!("{} Bulk send: {} emails, delay {}s-{}s", 
        ROCKET, 
        style(emails.len()).cyan(),
        style(min_delay).yellow(),
        style(max_delay).yellow()
    );
    
    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Confirmar envio?")
        .default(true)
        .interact()? 
    {
        println!("Cancelado!");
        return Ok(());
    }
    
    let pb = ProgressBar::new(emails.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:30.cyan/blue}] {pos}/{len} {msg}")?
            .progress_chars("█▓░"),
    );
    
    let mut success = 0;
    let mut failed = 0;
    
    for (i, email) in emails.iter().enumerate() {
        pb.set_message(format!("→ {}", email));
        
        let result = send_email(config, email, cv).await;
        
        let record = SentRecord {
            email: email.clone(),
            sent_at: Local::now(),
            success: result.is_ok(),
            error: result.as_ref().err().map(|e| e.to_string()),
        };
        log.records.push(record);
        save_log(log)?;
        
        match result {
            Ok(_) => {
                success += 1;
                pb.println(format!("  {} {}", CHECK, style(email).green()));
            }
            Err(e) => {
                failed += 1;
                pb.println(format!("  {} {} - {}", CROSS, style(email).red(), e));
            }
        }
        
        pb.inc(1);
        
        // delay random entre envios (exceto no último)
        if i < emails.len() - 1 {
            let delay = rand::thread_rng().gen_range(min_delay..=max_delay);
            pb.set_message(format!("Aguardando {}s...", delay));
            tokio::time::sleep(Duration::from_secs(delay)).await;
        }
    }
    
    pb.finish_with_message("Concluído!");
    
    println!();
    println!("{} Resultado: {} enviados, {} falhados", 
        SPARKLE,
        style(success).green().bold(),
        style(failed).red().bold()
    );
    
    Ok(())
}

fn view_log(log: &SentLog) {
    if log.records.is_empty() {
        println!("{} Nenhum email enviado ainda.", MAIL);
        return;
    }
    
    println!();
    println!("{} {} emails no histórico:", MAIL, style(log.records.len()).cyan());
    println!("{}", style("─".repeat(60)).dim());
    
    for r in log.records.iter().rev().take(20) {
        let status = if r.success { 
            style("OK").green() 
        } else { 
            style("FAIL").red() 
        };
        println!("  [{}] {} - {}", status, r.sent_at.format("%d/%m %H:%M"), r.email);
    }
    println!("{}", style("─".repeat(60)).dim());
}

fn preview_email(config: &Config) {
    let (subj, body) = build_email(config);
    
    println!();
    println!("{} Preview do email:", MAIL);
    println!("{}", style("─".repeat(50)).dim());
    println!("{}: {}", style("Subject").cyan(), subj);
    println!("{}", style("─".repeat(50)).dim());
    println!("{}", body);
    println!("{}", style("─".repeat(50)).dim());
    println!("{}: cv.pdf", style("Anexo").cyan());
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    print_banner();
    
    // check config exists
    if !std::path::Path::new(CONFIG_FILE).exists() {
        println!("{} config.json não encontrado!", CROSS);
        return Ok(());
    }
    
    let config = load_config()?;
    println!("{} Config carregado: {}", CHECK, style(&config.profile.name).green());
    
    // check cv exists
    if !std::path::Path::new(CV_FILE).exists() {
        println!("{} cv.pdf não encontrado! Coloca o ficheiro na pasta.", CROSS);
        return Ok(());
    }
    let cv = load_cv()?;
    println!("{} CV carregado: {}KB", CHECK, style(cv.len() / 1024).cyan());
    
    let mut log = load_log();
    print_stats(&log);
    
    loop {
        let options = vec![
            "📧 Enviar single (1 email)",
            "🚀 Enviar bulk (vários emails)",
            "👁️  Preview do email",
            "📋 Ver histórico",
            "❌ Sair",
        ];
        
        let sel = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("O que queres fazer?")
            .items(&options)
            .default(0)
            .interact()?;
        
        println!();
        
        match sel {
            0 => send_single(&config, &cv, &mut log).await?,
            1 => send_bulk(&config, &cv, &mut log).await?,
            2 => preview_email(&config),
            3 => view_log(&log),
            4 => {
                println!("{} Até a próxima mano!", SPARKLE);
                break;
            }
            _ => {}
        }
        
        println!();
    }
    
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs, io::Write};
    use tempfile::TempDir;

    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        let config = Config {
            profile: Profile {
                name: "João Silva".to_string(),
                email: "joao@example.com".to_string(),
                phone: "+351 912 345 678".to_string(),
                title: "Desenvolvedor Rust".to_string(),
                summary: "Desenvolvedor experiente".to_string(),
                skills: vec!["Rust".to_string(), "Tokio".to_string()],
                experience_years: 5,
                linkedin: Some("linkedin.com/in/joao".to_string()),
                github: Some("github.com/joao".to_string()),
            },
            smtp: SmtpConfig {
                host: "smtp.example.com".to_string(),
                port: 587,
            },
            template: EmailTemplate {
                subject: "Candidatura - {{name}} - {{title}}".to_string(),
                body: "Olá,\nNome: {{name}}\nEmail: {{email}}\nSkills: {{skills}}\nLinkedIn: {{linkedin}}".to_string(),
            },
        };

        let config_path = temp_dir.path().join(CONFIG_FILE);
        let config_json = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&config_path, config_json).unwrap();

        let cv_path = temp_dir.path().join(CV_FILE);
        let mut cv_file = fs::File::create(cv_path).unwrap();
        cv_file.write_all(b"%PDF-1.4 fake pdf content").unwrap();

        let log_path = temp_dir.path().join(LOG_FILE);
        fs::write(&log_path, "{\"records\":[]}").unwrap();

        temp_dir
    }

    macro_rules! with_temp_dir {
        ($temp_dir:expr, $body:expr) => {{
            let old_dir = std::env::current_dir().unwrap();
            std::env::set_current_dir($temp_dir.path()).unwrap();
            let result = $body;
            std::env::set_current_dir(old_dir).unwrap();
            result
        }};
    }

    #[test]
    fn test_load_config() {
        let temp_dir = setup_test_env();
        with_temp_dir!(temp_dir, {
            let config = load_config().unwrap();
            assert_eq!(config.profile.name, "João Silva");
            assert_eq!(config.profile.title, "Desenvolvedor Rust");
            assert_eq!(config.smtp.port, 587);
        });
    }

    #[test]
    fn test_load_cv() {
        let temp_dir = setup_test_env();
        with_temp_dir!(temp_dir, {
            let cv = load_cv().unwrap();
            assert!(cv.len() > 10);
        });
    }

    #[test]
    fn test_load_log_empty() {
        let temp_dir = setup_test_env();
        with_temp_dir!(temp_dir, {
            let log = load_log();
            assert!(log.records.is_empty());
        });
    }

    #[test]
    fn test_save_and_load_log() {
        let temp_dir = setup_test_env();
        with_temp_dir!(temp_dir, {
            let mut log = SentLog::default();
            log.records.push(SentRecord {
                email: "test@example.com".to_string(),
                sent_at: Local::now(),
                success: true,
                error: None,
            });
            save_log(&log).unwrap();

            let loaded = load_log();
            assert_eq!(loaded.records.len(), 1);
            assert_eq!(loaded.records[0].email, "test@example.com");
            assert!(loaded.records[0].success);
        });
    }

    #[test]
    fn test_build_email() {
        let temp_dir = setup_test_env();
        with_temp_dir!(temp_dir, {
            let config = load_config().unwrap();
            let (subject, body) = build_email(&config);

            assert_eq!(subject, "Candidatura - João Silva - Desenvolvedor Rust");
            assert!(body.contains("João Silva"));
            assert!(body.contains("joao@example.com"));
            assert!(body.contains("Rust, Tokio"));
            assert!(body.contains("linkedin.com/in/joao"));
        });
    }

    #[test]
    fn test_build_email_with_missing_optionals() {
        let config = Config {
            profile: Profile {
                name: "Ana".to_string(),
                email: "ana@example.com".to_string(),
                phone: "123".to_string(),
                title: "Dev".to_string(),
                summary: "Sum".to_string(),
                skills: vec![],
                experience_years: 3,
                linkedin: None,
                github: None,
            },
            smtp: SmtpConfig {
                host: "host".to_string(),
                port: 25,
            },
            template: EmailTemplate {
                subject: "{{name}} - {{title}}".to_string(),
                body: "{{linkedin}} {{github}} {{experience_years}}".to_string(),
            },
        };

        let (subject, body) = build_email(&config);
        assert_eq!(subject, "Ana - Dev");
        assert_eq!(body, "N/A N/A 3");
    }

    #[test]
    fn test_get_smtp_creds_success() {
        env::set_var("SMTP_USER", "user@test.com");
        env::set_var("SMTP_PASS", "secret");
        get_smtp_creds().unwrap();
        env::remove_var("SMTP_USER");
        env::remove_var("SMTP_PASS");
    }

    #[test]
    fn test_get_smtp_creds_missing_user() {
        env::remove_var("SMTP_USER");
        env::remove_var("SMTP_PASS");
        assert!(get_smtp_creds().is_err());
    }

    #[test]
    fn test_get_smtp_creds_missing_pass() {
        env::set_var("SMTP_USER", "user@test.com");
        env::remove_var("SMTP_PASS");
        assert!(get_smtp_creds().is_err());
        env::remove_var("SMTP_USER");
    }

    #[test]
    fn test_load_config_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        with_temp_dir!(temp_dir, {
            assert!(load_config().is_err());
        });
    }

    #[test]
    fn test_load_cv_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        with_temp_dir!(temp_dir, {
            assert!(load_cv().is_err());
        });
    }
}