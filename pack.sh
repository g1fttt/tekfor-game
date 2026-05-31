mkdir out

cargo build --release

mv target/release/tekfor-game out/
cp -r assets game_api levels scripts .luarc.json out/

zip out.zip out/ -r

rm -rf out
