#!/bin/bash

PASSWORD=`head -c1000 /dev/urandom | tr -dc [:alpha:][:digit:] | head -c 16; echo ;`
JWT_SECRET=`head -c1000 /dev/urandom | tr -dc [:alpha:][:digit:] | head -c 32; echo ;`
SECRET_KEY=`head -c1000 /dev/urandom | tr -dc [:alpha:][:digit:] | head -c 32; echo ;`
DB=podcatch

sudo apt-get install -y postgresql

sudo -u postgres createuser -E -e $USER
sudo -u postgres psql -c "CREATE ROLE $USER PASSWORD '$PASSWORD' NOSUPERUSER NOCREATEDB NOCREATEROLE INHERIT LOGIN;"
sudo -u postgres psql -c "ALTER ROLE $USER PASSWORD '$PASSWORD' NOSUPERUSER NOCREATEDB NOCREATEROLE INHERIT LOGIN;"
sudo -u postgres createdb $DB

cat > ${HOME}/.config/sync_app_rust/config.env <<EOL
DATABASE_URL=postgresql://$USER:$PASSWORD@localhost:5432/$DB
EOL

psql $DB < scripts/podcasts.sql
psql $DB < scripts/episodes.sql