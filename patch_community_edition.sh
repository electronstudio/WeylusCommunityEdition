git clone https://github.com/H-M-H/Weylus.git
cd Weylus
echo "CONTRIBUTORS merge=ours" >.gitattributes
git config --global merge.ours.driver true

git remote add qdlmcfresh https://github.com/qdlmcfresh/Weylus.git
git fetch qdlmcfresh stylus_windows
git merge qdlmcfresh/stylus_windows -m "merged qdlmcfresh"

git remote add qdlmcfreshcompat https://github.com/electronstudio2/WeylusQdlmcFreshLyonbotCompat.git
git fetch qdlmcfreshcompat stylus_windows
git merge qdlmcfreshcompat/stylus_windows -m "merged qdlmcfresh compatability"

git remote add OmegaRogue https://github.com/OmegaRogue/Weylus.git
git fetch OmegaRogue master
git merge OmegaRogue/master -m "merged OmegaRogue"

git remote add scribblemaniac https://github.com/scribblemaniac/Weylus.git
git fetch scribblemaniac fix-qr-code
git merge scribblemaniac/fix-qr-code -m "merged scribblemaniac"

git remote add Diordany https://github.com/Diordany/Weylus.git
git fetch Diordany pr-func-proto
git merge Diordany/pr-func-proto -m "merged Diordany"


git remote add lyonbot https://github.com/lyonbot/Weylus.git
git fetch lyonbot pr
git merge lyonbot/pr -m "merged lyonbot"


git remote add electronstudio2 https://github.com/electronstudio2/Weylus.git
git fetch electronstudio2 build-fixes
git merge electronstudio2/build-fixes -m "merged build-fixes"


sed -i '/<<<<<<< HEAD/,/=======/d' .github/workflows/build.yml
sed -i '/>>>>>>> electronstudio2\/build-fixes/d' .github/workflows/build.yml

sed -i '/<<<<<<< HEAD/d' build.rs
sed -i '/=======/,/>>>>>>> electronstudio2\/build-fixes/d' build.rs
sed -i '/>>>>>>> electronstudio2\/build-fixes/d' build.rs

git add build.rs .github/workflows/build.yml
git commit -m 'resolve conflict'

git fetch electronstudio2 community-edition-patches
git merge electronstudio2/community-edition-patches -m "merged community-edition-patches"

git remote add wce https://github.com/electronstudio/WeylusCommunityEdition.git
###git push -u wce master --force
