#!/bin/bash

DB="podcatch"
BUCKET="podcatch-db-backup"

TABLES="
episodes
podcasts
google_music_metadata
"

mkdir -p backup/

for T in $TABLES;
do
    aws s3 cp s3://${BUCKET}/${T}.sql.gz backup/${T}.sql.gz
    gzip -dc backup/${T}.sql.gz | psql $DB -c "COPY $T FROM STDIN"
done
