git clone https://github.com/H-M-H/Weylus.git
cd Weylus

git remote add qdlmcfresh https://github.com/qdlmcfresh/Weylus.git
git fetch qdlmcfresh stylus_windows
git merge qdlmcfresh/stylus_windows -m "merged qdlmcfresh"

git remote add OmegaRogue https://github.com/OmegaRogue/Weylus.git
git fetch OmegaRogue master
git merge -s recursive -X theirs OmegaRogue/master -m "merged OmegaRogue"

git remote add scribblemaniac https://github.com/scribblemaniac/Weylus.git
git fetch scribblemaniac fix-qr-code
git merge scribblemaniac/fix-qr-code -m "merged scribblemaniac"

git remote add Diordany https://github.com/Diordany/Weylus.git
git fetch Diordany pr-func-proto
git merge Diordany/pr-func-proto -m "merged Diordany"

git remote add lyonbot https://github.com/lyonbot/Weylus.git
git fetch lyonbot pr
git merge lyonbot/pr -m "merged lyonbot"
