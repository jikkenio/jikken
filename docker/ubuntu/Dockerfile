FROM ubuntu:22.04
LABEL maintainer="Jikken <https://github.com/jikkenio/jikken/issues>"

ENV jk_args=""
ENV run_args=""

RUN apt update
RUN apt install -y curl
RUN curl https://jikken.io/install.sh | bash

# jk run ${args} jk_tests is the default command.
# This allows you to mount your local tests to /etc/jk_tests and be done with it
#
# docker run -v ~/git/my_repo/tests/jk:/etc/jk_tests
#
# a more involved example
#
# 
# docker run -v ~/git/my_repo/tests/jk:/etc/jk_tests [image] -e jk_args="--trace" -e JIKKEN_API_KEY="SUPER_SECRET_API_KEY" -e run_args="-t regression"
ENTRYPOINT /jk ${jk_args} run ${run_args} /etc/jk_tests