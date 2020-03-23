FROM ubuntu:latest

RUN apt-get -y update
RUN apt-get -y upgrade
RUN apt-get install -y sqlite3 libsqlite3-dev
WORKDIR /usr/src/rates_converter
COPY ./target/release/warp-currency /usr/src/rates_converter
COPY ./target/release/rates.db /usr/src/rates_converter
RUN cd /usr/src/rates_converter

CMD ["./warp-currency"]
