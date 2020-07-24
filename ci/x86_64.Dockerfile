FROM rustembedded/cross:x86_64-unknown-linux-gnu

RUN apt-get update && \
    apt-get install --assume-yes libncurses5-dev libncursesw5-dev libsqlite3-dev
