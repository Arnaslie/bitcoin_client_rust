# put this file, add_test.py, and your netid.zip file in a new directory
cwd=$(pwd)
for zipfile in *.zip; do
    cd $cwd
    netid=${zipfile%%.*}
    unzip -qq $zipfile -d $netid
    if [ -d $netid ]; then
		echo "student netid: $netid" >> log.txt
		python3 add_test.py $netid/ece598pv-sp2022-main
		cd $netid/ece598pv-sp2022-main
		cargo test sp2022autograder01 >> ../../log.txt 2>> build_log.txt
    fi
done
#grep 'student netid\|test result' log.txt > result.txt