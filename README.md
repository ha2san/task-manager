# Task Manager

A full-stack habit tracking and task management application built with Rust and vanilla JavaScript. This system allows users to create recurring tasks based on days of the week, track daily completions, and visualize productivity through a 30-day activity heatmap. 

## Features

* **User Authentication**: Secure registration and login using Argon2 password hashing and JWT-based session management. 


* **Recurring Task Management**: Define tasks that repeat on specific days of the week. 


* **Daily Tracking**: A dashboard specifically for "Today" that displays only relevant tasks for the current day. 


* **Productivity Analytics**: A heatmap visualization of the last 30 days and global statistics on task completion. 


* **Task Organization**: Capabilities for soft-deleting, archiving, and filtering tasks by title or scheduled days. 


* **Data Portability**: Import tasks from JSON files for quick setup. 



## Technical Stack

### Backend

* **Language**: Rust (Edition 2024) 


* **Framework**: Axum 


* **Database**: PostgreSQL with SQLx for type-safe asynchronous queries. 


* **Authentication**: Argon2 (hashing) and jsonwebtoken (JWT). 



### Frontend

* **Logic**: Vanilla JavaScript with asynchronous fetch operations. 


* **Styling**: Custom CSS with a responsive design for mobile and desktop. 


* **Visualization**: Dynamic heatmap rendering based on completion percentages. 



### Infrastructure

* **Containerization**: Docker and Docker Compose. 


* **Migrations**: Internal database migration handling on startup. 



## Getting Started

### Prerequisites

* Docker and Docker Compose
* A `.env` file in the root directory (refer to the environment configuration section)

### Installation

1. Clone the repository and ensure you are in the project root.
2. Configure your `.env` file:
```env
DATABASE_URL=postgres://task:task@db:5432/taskdb
JWT_SECRET=your_random_secret_string
POSTGRES_USER=task
POSTGRES_PASSWORD=task
POSTGRES_DB=taskdb

```


3. Launch the application using Docker Compose:
```bash
docker-compose up --build

```


4. Access the application at `http://localhost:3000`.

## API Routes

### Authentication

* `POST /api/auth/register`: Create a new user account. 


* 
`POST /api/auth/login`: Authenticate and receive a JWT. 



### Tasks

* `GET /api/tasks`: Retrieve tasks scheduled for the current date. 


* `GET /api/tasks/all`: Retrieve all non-deleted tasks for the user. 


* `POST /api/tasks`: Create a new task with recurrence days. 


* `POST /api/tasks/:id/toggle`: Toggle the completion status for today. 


* `PATCH /api/tasks/:id`: Archive or activate a task. 


* `DELETE /api/tasks/:id`: Soft-delete a task. 



### Analytics

* `GET /api/stats`: Retrieve 30-day history and global completion totals. 



## Project Structure

```text
.
├── backend                 # Rust source code
│   ├── migrations          # SQL initialization scripts
│   └── src                 # API logic, models, and middleware
├── frontend                # Web interface (HTML, CSS, JS)
├── docker-compose.yml      # Orchestration
└── Dockerfile              # Multi-stage build for Rust and Frontend

```

