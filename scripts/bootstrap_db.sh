#!/bin/bash

if [ -z "$PASSWORD" ]; then
    PASSWORD=`head -c1000 /dev/urandom | tr -dc [:alpha:][:digit:] | head -c 16; echo ;`
fi
DB=podcatch

sudo apt-get install -y postgresql

sudo -u postgres createuser -E -e $USER
sudo -u postgres psql -c "CREATE ROLE $USER PASSWORD '$PASSWORD' NOSUPERUSER NOCREATEDB NOCREATEROLE INHERIT LOGIN;"
sudo -u postgres psql -c "ALTER ROLE $USER PASSWORD '$PASSWORD' NOSUPERUSER NOCREATEDB NOCREATEROLE INHERIT LOGIN;"
sudo -u postgres createdb $DB

cat > ${HOME}/.config/podcatch_rust/config.env <<EOL
DATABASE_URL=postgresql://$USER:$PASSWORD@localhost:5432/$DB
EOL

cat > ${HOME}/.config/podcatch_rust/postgres.toml <<EOL
[podcatch_rust]
database_url = 'postgresql://$USER:$PASSWORD@localhost:5432/$DB'
destination = 'file:///home/ddboline/setup_files/build/podcatch_rust/backup'
tables = ['episodes', 'podcasts', 'google_music_metadata']
EOL

psql $DB < scripts/podcasts.sql
psql $DB < scripts/episodes.sql
psql $DB < scripts/google_music_metadata.sql
