# TaskFlow Manager

TaskFlow est une application de gestion de tâches quotidiennes et de suivi d'habitudes. Elle permet de planifier des tâches récurrentes selon les jours de la semaine, de suivre leur complétion en temps réel et de visualiser sa productivité sur les 30 derniers jours grâce à une carte thermique.

## Fonctionnalités

* 
**Authentification sécurisée** : Inscription et connexion avec hachage des mots de passe via Argon2 et gestion des sessions par jetons JWT.


* 
**Gestion des tâches** : Création de tâches avec sélection des jours de récurrence (1 à 7 pour Lundi à Dimanche).


* 
**Tableau de bord quotidien** : Affichage dynamique des tâches prévues pour la date actuelle.


* 
**Statistiques de productivité** : Visualisation de l'historique de complétion et calcul du taux de réussite global.


* 
**Importation de données** : Possibilité d'importer massivement des tâches via un fichier au format JSON.


* 
**Architecture robuste** : Backend développé en Rust avec Axum et SQLx pour des performances optimales.



## Stack Technique

* 
**Backend** : Rust (Edition 2024), Axum, SQLx.


* 
**Base de données** : PostgreSQL.


* 
**Frontend** : Vanilla JavaScript, HTML5, CSS3.


* 
**Conteneurisation** : Docker, Docker Compose.



## Installation

### Prérequis

* Docker et Docker Compose
* Un fichier `.env` configuré à la racine (voir section Configuration)

### Procédure

1. Clonez le dépôt :
```bash
git clone <url-du-depot>
cd task-manager

```


2. Lancez l'application avec Docker Compose :
```bash
docker-compose up --build

```


3. L'application est accessible à l'adresse suivante : `http://localhost:3000`.



## Configuration

Un fichier `.env` est nécessaire au bon fonctionnement de l'application. Vous pouvez utiliser le modèle suivant :

```env
# Configuration Base de données
DATABASE_URL=postgres://task:task@db:5432/taskdb
POSTGRES_USER=task
POSTGRES_PASSWORD=task
POSTGRES_DB=taskdb

# Sécurité
JWT_SECRET=votre_cle_secrete_longue_et_aleatoire

# Chemins
FRONTEND_PATH=/app/frontend

```

## Structure du Projet

* 
`/backend` : Code source Rust, migrations SQL et logique métier.


* 
`/frontend` : Fichiers statiques (HTML, CSS, JS) servis par le backend.


* 
`Dockerfile` : Configuration de l'image de production multi-étape basée sur Debian.

