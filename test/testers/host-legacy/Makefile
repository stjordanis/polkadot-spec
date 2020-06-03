.PHONY: all clean

all: genesis-legacy.json genesis-legacy.yaml genesis-legacy.kagome.json

genesis-legacy.json:
	cargo -Zfeatures=build_dep,compare run --release > $@

genesis-legacy.yaml: genesis-legacy.json
	yq -y < $< '.genesis.raw[0] | { keys: keys_unsorted | map(.[2:]), values: map(.[2:]) }' > $@

genesis-legacy.kagome.json: genesis-legacy.json
	jq < $< '.genesis.raw = .genesis.raw[0]' > $@

clean:
	rm -rf target genesis-legacy.json genesis-legacy.yaml genesis-legacy.kagome.json
