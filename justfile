serve:
    cd draftsmith_flask && \
    CSRF_SECRET_KEY="832983289329832" API_SCHEME=http API_PORT=37240 API_HOST=vidar poetry run gunicorn -w 4 -b 0.0.0.0:5000 server:app

serve-dev:
    cd draftsmith_flask && \
    CSRF_SECRET_KEY="832983289329832" API_SCHEME=http API_PORT=37240 API_HOST=vidar poetry run python server.py

watch:
    # Because the templates are compiled into the binary
    # It's easier to recompile with entr
    fd -t f html | entr -r cargo run -- serve --api-host eir --api-port 37242 -p 5000

sass:
    cd static
    npx sass --no-source-map static/styles:static/css
    build/compile_css.py

format:
    ruff format      src
    ruff check --fix src

install-deps:
    # poetry install
    npm install
    mkdir -p ./src/static/css/ ./src/static/js/
    cp -r ./node_modules/katex ./src/static/

tailwind-init:
    npm install -D tailwindcss
    npx tailwindcss init

echo:
    echo "Hello $name"
