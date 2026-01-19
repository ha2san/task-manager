-- =========================
-- USERS
-- =========================
CREATE TABLE users (
    id UUID PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

-- =========================
-- TASKS
-- =========================
CREATE TABLE tasks (
    id SERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    active BOOLEAN NOT NULL DEFAULT true,
    deleted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

-- =========================
-- TASK DAYS
-- =========================
CREATE TABLE task_days (
    task_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
    day_of_week INTEGER NOT NULL CHECK (day_of_week BETWEEN 1 AND 7),
    PRIMARY KEY (task_id, day_of_week)
);

-- =========================
-- TASK COMPLETIONS
-- =========================
CREATE TABLE task_completions (
    task_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    completed BOOLEAN NOT NULL DEFAULT true,
    PRIMARY KEY (task_id, date)
);

-- =========================
-- INDEXES
-- =========================
CREATE INDEX idx_tasks_user_id ON tasks(user_id);
CREATE INDEX idx_task_days_day ON task_days(day_of_week);
CREATE INDEX idx_task_completions_date ON task_completions(task_id, date);

