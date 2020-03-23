FROM ubuntu

WORKDIR /usr/src/rates_converter
COPY ./target/release/warp-currency /usr/src/rates_converter
COPY ./target/release/rates.db /usr/src/rates_converter
RUN cd /usr/src/rates_converter

CMD ["./warp-currency"]