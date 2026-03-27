FROM alpine:3.21

RUN apk add --no-cache net-snmp bash gawk

EXPOSE 161/udp

ENTRYPOINT ["snmpd", "-f", "-Lo", "-C", "-c"]
CMD ["/etc/snmp-test/snmpd.conf"]
