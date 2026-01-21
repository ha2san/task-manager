# =========================
# 1. Builder - Utilisation de Nightly pour supporter l'Edition 2024
# =========================
FROM rustlang/rust:nightly-bookworm AS builder

WORKDIR /usr/src/app

# Dépendances système
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copie des fichiers
COPY backend/Cargo.toml backend/Cargo.lock ./backend/
COPY backend/src ./backend/src
COPY backend/migrations ./backend/migrations
COPY backend/.sqlx ./backend/.sqlx
COPY frontend ./frontend
COPY wait-for-it.sh ./wait-for-it.sh

WORKDIR /usr/src/app/backend

# On active le mode hors-ligne pour SQLx
ENV SQLX_OFFLINE=true

# Compilation avec Nightly
RUN cargo build --release

# =========================
# 2. Runtime (Image finale légère)
# =========================
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        libpq5 \
    && rm -rf /var/lib/apt/lists/*

# On récupère le binaire (Vérifie bien le nom "task-manager" dans ton Cargo.toml)
COPY --from=builder /usr/src/app/backend/target/release/task-manager .

# On récupère le dossier frontend
COPY --from=builder /usr/src/app/frontend ./frontend

# On récupère le script d'attente
COPY --from=builder /usr/src/app/wait-for-it.sh ./wait-for-it.sh
RUN chmod +x wait-for-it.sh

# Configuration
#ENV DATABASE_URL=postgres://task:task@db:5432/taskdb
#ENV SQLX_OFFLINE=true 

EXPOSE 3000

# Lancement (Ton code doit gérer les migrations en interne maintenant)
CMD ["./task-manager"]
