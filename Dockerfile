FROM alpine:3.17
ARG deployable
COPY target/release/$deployable /bin/$deployable
EXPOSE 60000

RUN addgroup -S $deployable && adduser -S $deployable -G $deployable
USER $deployable
RUN ["$deployable"]



