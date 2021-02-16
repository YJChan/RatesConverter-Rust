# RatesConverter-Rust

## Description
This repo is a rewrite of https://github.com/YJChan/RatesConverter.git which written in Node.js previously.
I've choosen Rust for its lightweight and speed, Warp for the web framework. 
This project is aim to support this Mozilla Web Extension https://github.com/YJChan/Immediate-Currency-Converter.git.

To run this project,
You need to <br>
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

source $HOME/.cargo/env

sudo apt-get install -y libsqlite3-dev

<br>
<code>git clone https://github.com/YJChan/RatesConverter-Rust.git</code>
<br>
<code>cargo run</code>
<br>
<br>
-- Experimental Feature --
<br>
Live currency rates with 5 to 10 secs interval. Subscribing from API would be very expensive to achieve a live currency rate with 5 to 10 secs interval. So, I plan to scrape it from some where else, then using event stream to stream the data back to client.

1st, Open a channel to subcribe for scaped data

2nd, send it with tokio async task

3rd, stream back using SSE (Server-Side Event)

<br>
If you find any other way of making it more efficent, other than subscribing expensive API or scaping data. Please let me know either email or opening a issue.
<br>
<br>
Thanks!
