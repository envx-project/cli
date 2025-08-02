generate: openapi.json openapi-generator-config.json
	openapi-generator generate -i openapi.json -g rust -c openapi-generator-config.json -o ./envx_sdk --skip-validate-spec
