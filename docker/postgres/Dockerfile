FROM pgvector/pgvector:pg16

ARG AGE_BRANCH=PG16
ARG PG_MAJOR=16

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bison \
        build-essential \
        ca-certificates \
        flex \
        git \
        postgresql-server-dev-${PG_MAJOR} \
    && git clone --depth 1 --branch "${AGE_BRANCH}" https://github.com/apache/age.git /tmp/age \
    && make -C /tmp/age PG_CONFIG=/usr/lib/postgresql/${PG_MAJOR}/bin/pg_config install \
    && mkdir -p /usr/lib/postgresql/${PG_MAJOR}/lib/plugins \
    && ln -sf /usr/lib/postgresql/${PG_MAJOR}/lib/age.so /usr/lib/postgresql/${PG_MAJOR}/lib/plugins/age.so \
    && rm -rf /tmp/age /var/lib/apt/lists/*
