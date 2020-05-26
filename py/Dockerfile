# Dockerfile for building linux targets

FROM rust:1-buster

RUN apt-get update && \
	apt-get install -y \
	build-essential \
	libssl-dev \
	zlib1g-dev \
	libbz2-dev \
	libreadline-dev \
	libsqlite3-dev \
	wget \
	curl \
	llvm \
	libncurses5-dev \
	libncursesw5-dev \
	xz-utils \
	tk-dev \
	libffi-dev \
	liblzma-dev \
	python-openssl \
	git

RUN curl https://pyenv.run | bash
ENV PATH="${HOME}/.pyenv/bin:${PATH}"

RUN eval "$(pyenv init -)"; \
	eval "$(pyenv virtualenv-init -)"; \
	pyenv install 3.6.9
RUN eval "$(pyenv init -)"; \
	eval "$(pyenv virtualenv-init -)"; \
	pyenv install 3.7.4
RUN eval "$(pyenv init -)"; \
	eval "$(pyenv virtualenv-init -)"; \
	pyenv install 3.8.2



