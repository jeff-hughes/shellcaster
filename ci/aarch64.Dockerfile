FROM rustembedded/cross:aarch64-unknown-linux-gnu

RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install --assume-yes libncurses5-dev:arm64 libncursesw5-dev:arm64 libsqlite3-dev:arm64