# RMenu Installation/Deployment Configuration

CARGO=cargo
FLAGS=--release

DEST=$(HOME)/.config/wclipd

clean:
	${CARGO} clean

build:
	${CARGO} build ${FLAGS}

install:
	${CARGO} install --path .
	mkdir -p ${DEST}
	cp -f default-config.yaml ${DEST}/config.yaml
