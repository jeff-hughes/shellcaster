FROM rustembedded/cross:i686-unknown-linux-gnu

RUN dpkg --add-architecture i386 && \
    apt-get update && \
    apt-get install --assume-yes libncurses5-dev:i386 libncursesw5-dev:i386 libsqlite3-dev:i386
