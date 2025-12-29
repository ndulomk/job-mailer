# Job Mailer CLI

Ferramenta de linha de comando para envio automatizado de candidaturas de emprego via email.

## Requisitos

- Rust 1.70+
- Conta de email com SMTP habilitado (Gmail, Outlook, etc.)

## Instalacao

```bash
git clone <repo>
cd job-mailer
cargo build --release
```

O executavel sera gerado em `./target/release/job-mailer`.

### Instalacao global (Linux)

```bash
sudo cp ./target/release/job-mailer /usr/local/bin/
```

### Cross-compile para Windows

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

## Configuracao

### Estrutura de ficheiros

```
job-mailer/
├── job-mailer       # executavel
├── config.json      # perfil e template
├── .env             # credenciais SMTP
├── cv.pdf           # curriculo
└── sent_log.json    # log de envios (gerado automaticamente)
```

### 1. Ficheiro .env

Contem as credenciais SMTP. Nunca versionar este ficheiro.

```
SMTP_USER=seu-email@gmail.com
SMTP_PASS=sua-app-password
```

#### Obter App Password (Gmail)

1. Aceder a Google Account > Security
2. Activar 2-Factor Authentication
3. Em "App passwords", gerar nova password
4. Usar a password gerada no campo SMTP_PASS

### 2. Ficheiro config.json

```json
{
  "profile": {
    "name": "Nome Completo",
    "email": "seu@email.com",
    "phone": "+244 923 456 789",
    "title": "Cargo Pretendido",
    "summary": "Descricao profissional breve.",
    "skills": ["Skill1", "Skill2", "Skill3"],
    "experience_years": 5,
    "linkedin": "https://linkedin.com/in/seu-perfil",
    "github": "https://github.com/seu-usuario"
  },
  "smtp": {
    "host": "smtp.gmail.com",
    "port": 587
  },
  "template": {
    "subject": "Candidatura - {{title}} - {{name}}",
    "body": "Corpo do email com placeholders"
  }
}
```

#### Placeholders disponiveis

| Placeholder | Descricao |
|-------------|-----------|
| `{{name}}` | Nome completo |
| `{{email}}` | Email |
| `{{phone}}` | Telefone |
| `{{title}}` | Cargo/titulo |
| `{{summary}}` | Descricao profissional |
| `{{skills}}` | Lista de skills separadas por virgula |
| `{{experience_years}}` | Anos de experiencia |
| `{{linkedin}}` | URL do LinkedIn |
| `{{github}}` | URL do GitHub |

### 3. Ficheiro cv.pdf

Colocar o curriculo em formato PDF na mesma pasta do executavel com o nome `cv.pdf`.

## Utilizacao

```bash
./job-mailer
```

### Menu principal

```
O que queres fazer?
> Enviar single (1 email)
  Enviar bulk (varios emails)
  Preview do email
  Ver historico
  Sair
```

### Envio single

Envia uma candidatura para um unico destinatario.

1. Seleccionar "Enviar single"
2. Inserir email do destinatario
3. Aguardar confirmacao de envio

### Envio bulk

Envia candidaturas para multiplos destinatarios com intervalo aleatorio entre envios.

1. Seleccionar "Enviar bulk"
2. Inserir emails (um por linha, linha vazia para terminar)
3. Definir delay minimo entre envios (segundos)
4. Definir delay maximo entre envios (segundos)
5. Confirmar envio

O intervalo aleatorio entre envios reduz a probabilidade de deteccao como spam.

### Preview

Visualiza o email que sera enviado com todos os placeholders substituidos.

### Historico

Lista os ultimos 20 emails enviados com status (OK/FAIL) e data/hora.

## Ficheiro de log

O ficheiro `sent_log.json` regista todos os envios:

```json
{
  "records": [
    {
      "email": "destino@empresa.com",
      "sent_at": "2024-01-15T10:30:00",
      "success": true,
      "error": null
    }
  ]
}
```

## Configuracao SMTP por provider

| Provider | Host | Port |
|----------|------|------|
| Gmail | smtp.gmail.com | 587 |
| Outlook | smtp-mail.outlook.com | 587 |
| Yahoo | smtp.mail.yahoo.com | 587 |
| Zoho | smtp.zoho.com | 587 |

## Gitignore recomendado

```
.env
sent_log.json
cv.pdf
target/
```

## Resolucao de problemas

### Erro: "SMTP_USER not set in .env"

O ficheiro .env nao existe ou nao contem as variaveis necessarias.

### Erro: "config.json not found"

Executar a aplicacao uma vez para gerar o ficheiro padrao, ou criar manualmente.

### Erro: "cv.pdf not found"

Colocar o ficheiro cv.pdf na mesma pasta do executavel.

### Erro de autenticacao SMTP

- Verificar credenciais no .env
- Para Gmail, usar App Password (nao a password normal)
- Verificar se 2FA esta activo na conta

### Emails nao chegam ao destino

- Verificar pasta de spam do destinatario
- Aumentar intervalo entre envios no modo bulk
- Verificar se o email de origem esta validado no provider