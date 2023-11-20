FROM rust:1.73 as builder

RUN apt-get update 
RUN apt-get install -y ffmpeg tesseract-ocr mpv
RUN apt-get install -y libasound2 pulseaudio pavucontrol
RUN apt-get install -y libopencv-dev 
RUN apt-get install -y gdk+3.0
RUN apt-get install -y libatk1.0-dev
RUN apt-get install -y libpango-1.0-0
RUN apt-get install -y libgtk-3-dev
RUN apt-get install -y llvm-dev
RUN apt-get install -y clang
RUN apt-get install -y libclang-dev
RUN apt-get install -y libleptonica-dev
RUN apt-get install -y libtesseract-dev
RUN apt-get install -y libavfilter-dev
RUN apt-get install -y libavdevice-dev
RUN apt-get install -y libxcb-xfixes0-dev
RUN apt-get install -y libxcb-shape0-dev

RUN apt-get install -y yasm
RUN rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/myapp

COPY Cargo.* /usr/src/myapp/
RUN mkdir src && echo "// dummy file" > src/lib.rs && cargo fetch
COPY assets /usr/src/myapp/
COPY src/* /usr/src/myapp/src/
RUN cargo build -v --release --offline

# Create intall directory
RUN mkdir -p /opt/hg2jj/bin
RUN mkdir -p /opt/hg2jj/.cache
COPY assets /opt/hg2jj/assets

# Install
RUN cp /usr/src/myapp/target/release/hg2jj /opt/hg2jj/bin/hg2jj

ENTRYPOINT /opt/hg2jj/bin/hg2jj
