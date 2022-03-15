#!/bin/sh

set -eu

cd /srv

# yuk way of getting container ip without iproute2
#TODO: possibly use "inject_container_name" from dockertest framework
export CERT_IPSAN="$(awk '/32 host/ { print f } {f=$2}' </proc/net/fib_trie | grep -v 127.0.0.1 | head -n 1)"

# generate webserver privkey
openssl req -batch \
  -config openssl-csr.cnf \
  -newkey rsa:2048 \
  -sha256 \
  -nodes \
  -keyout webserver.key \
  -out webserver.csr \
  -outform PEM

echo -n ''> ca-db.txt
echo '01' >ca-serial.txt

# CA-sign webserver cert
openssl ca \
  -batch \
  -verbose \
  -days 30 \
  -config openssl-signing.cnf \
  -policy signing_policy \
  -extensions ca_extensions \
  -out webserver.crt \
  -infiles webserver.csr

chmod +r webserver.*
