# RMenu Installation/Deployment Configuration

CARGO=cargo
FLAGS=--release

DEST=$(HOME)/.config/wclipd
LOCAL_BIN=$(HOME)/.local/bin

notice:
	@echo "run 'make install'"

clean:
	${CARGO} clean

build:
	${CARGO} build ${FLAGS}

install: build
	mkdir -p ${DEST}
	cp -f default-config.yaml ${DEST}/config.yaml
	sudo install target/release/wclipd /usr/local/bin/.
	sudo install bin/wl-copy.sh /usr/local/bin/wl-copy
	sudo install bin/wl-paste.sh /usr/local/bin/wl-paste

uninstall:
	rm -rf ${DEST}
	sudo rm -f /usr/local/bin/wclipd
	sudo rm -f /usr/local/bin/wl-copy
	sudo rm -f /usr/local/bin/wl-paste
