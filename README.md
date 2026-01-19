# Task Manager

A modern, full-stack task management application designed for daily habit tracking and recurring routines. Built with a high-performance **Rust** backend and a lightweight **Vanilla JS** frontend, the application allows users to schedule tasks on specific days of the week and track daily completion status.

---

## 🚀 Key Features

* 
**Recurring Scheduling**: Create tasks that repeat only on specific days of the week.


* 
**Daily Tracking**: An "Aujourd'hui" (Today) view that shows only the tasks relevant to the current day.


* 
**Completion History**: Toggle tasks as complete/incomplete with persistent tracking in the database.


* 
**Full CRUD**: A dedicated management interface to edit task titles, update scheduled days, or delete tasks.


* 
**Secure Authentication**: User registration and login system featuring **Argon2** password hashing and **JWT** (JSON Web Token) authentication.


* 
**Modern UI**: A clean, responsive interface built with CSS variables, featuring a modal-based editing system and hover transitions.



---

## 🛠 Tech Stack

### Backend

* 
**Language**: Rust (Edition 2024).


* 
**Framework**: [Axum](https://github.com/tokio-rs/axum) for high-performance routing.


* 
**Database**: PostgreSQL with [SQLx](https://github.com/launchbadge/sqlx) for type-safe asynchronous queries.


* 
**Auth**: Argon2 for hashing and `jsonwebtoken` for secure sessions.


* 
**Runtime**: Tokio.



### Frontend

* 
**Architecture**: Vanilla JavaScript (ES6+) for zero-dependency speed.


* 
**Styling**: Custom CSS3 with Inter font and responsive layouts.


* 
**State**: LocalStorage-based JWT persistence.



### Infrastructure

* 
**Containerization**: Docker & Docker Compose for easy deployment.


* 
**Migrations**: Automated SQL migrations managed by the Rust backend on startup.



---

## 📦 Installation & Setup

### Prerequisites

* [Docker](https://www.docker.com/) and Docker Compose installed.

### Quick Start

1. **Clone the repository**:
```bash
git clone <your-repo-url>
cd task-manager

```


2. **Launch the application**:
```bash
docker-compose up --build

```


3. **Access the app**:
* Open your browser to `http://localhost:3000`.


* Register a new account (passwords must be at least 8 characters).





---

## 📂 Project Structure

```text
├── backend/
[cite_start]│   ├── migrations/    # SQL database schema [cite: 2-6]
│   ├── src/
[cite_start]│   │   ├── auth.rs    # JWT & Argon2 logic [cite: 7]
[cite_start]│   │   ├── db.rs      # SQLx pool & migrations [cite: 14]
[cite_start]│   │   ├── main.rs    # Server entry point [cite: 17]
[cite_start]│   │   ├── models.rs  # Rust structs for API [cite: 24]
[cite_start]│   │   ├── routes.rs  # Task CRUD handlers [cite: 34]
[cite_start]│   │   └── middleware.rs # JWT Auth guard [cite: 21]
├── frontend/
[cite_start]│   ├── app.js         # API communication logic [cite: 58]
[cite_start]│   ├── auth.html      # Login/Register page [cite: 71]
[cite_start]│   ├── index.html     # Today's task view [cite: 83]
[cite_start]│   ├── manage.html    # Full catalogue management [cite: 86]
[cite_start]│   └── style.css      # Custom UI design [cite: 104]
[cite_start]└── docker-compose.yml # Orchestration [cite: 57]

```

---

## 🛡 Environment Variables

The application can be configured via environment variables in the `docker-compose.yml`:

| Variable | Description | Default |
| --- | --- | --- |
| `DATABASE_URL` | Postgres connection string | <br>`postgres://task:task@db:5432/taskdb` 

 |
| `JWT_SECRET` | Secret key for token signing | <br>`changez_moi_en_production_123456789` 

 |
| `FRONTEND_PATH` | Path to serve static files | <br>`frontend` 

 |

Would you like me to add a section on the specific API endpoints or more detailed developer instructions?
