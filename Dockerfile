FROM debian:sid-slim
# RUN pip3 install torch==2.1.0 --index-url https://download.pytorch.org/whl/cpu
# RUN apt-get update -y \
#     && apt-get -y --no-install-recommends install curl build-essential\
#     && rm -rf /var/lib/apt/lists/* 
# RUN curl https://sh.rustup.rs -sSf | sh -s -- --profile minimal -y --default-toolchain nightly

# WORKDIR /app
# ADD hugedata ./hugedata
# ADD web_api ./web_api
# ADD web_types ./web_types
# ADD tch_tchotchkes ./tch_tchotchkes
# ADD fish_teacher ./fish_teacher
# ADD compact_board ./compact_board

# WORKDIR /app/web_api
# RUN /root/.cargo/bin/cargo build --release

# ENTRYPOINT ["/root/.cargo/bin/cargo", "run", "--release"]

#RUN cat /etc/apt/sources.list.d/debian.sources && exit 1
RUN sed -i 's/main/main/' /etc/apt/sources.list.d/debian.sources \
    && apt-get update -y \
    && apt-get -y --no-install-recommends install libgomp1 stockfish  \
    && rm -rf /var/lib/apt/lists/* 

WORKDIR /app
ADD hugedata ./hugedata

RUN mkdir -p target/release
ADD target/release/web_api ./target/release/web_api

COPY extra_lib /usr/lib

WORKDIR /app/target
#ENV LD_LIBRARY_PATH=/lib
ENV PART=/usr/games
ENTRYPOINT ["./release/web_api"]
