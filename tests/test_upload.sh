#!/bin/bash -eu

WORK=`mktemp -d`
mkdir ${WORK}/logs
mkdir ${WORK}/original
mkdir ${WORK}/actual
mkdir ${WORK}/actual/plaintext

cleanup () {
	rm -fr ${WORK}/logs
	rm -fr ${WORK}/original
	rm -fr ${WORK}/actual
	rm -fr ${WORK}/actual/plaintext
	rmdir ${WORK}
}

export GNUPGHOME=$(dirname $0)/gnupg
chmod 700 ${GNUPGHOME}
chmod 600 ${GNUPGHOME}/secring.gpg

# generate test data
for i in 1 2 3 4 ; do
	dd if=/dev/urandom of=${WORK}/original/data_$i bs=1024 count=10 2>/dev/null
	cp ${WORK}/original/data_$i ${WORK}/logs/
done

# perform upload
$(dirname $0)/../target/debug/log-upload \
	--log-dir ${WORK}/logs --s3-path ${LOG_UPLOAD_TEST_BUCKET}/test--log-upload/ --encrypt-key test--log-upload --signing-key test--log-upload \
	|| { cleanup ; exit 1 ; }

# download resulting test files
s3cmd -q --ssl --delete-after-fetch sync s3://${LOG_UPLOAD_TEST_BUCKET}/test--log-upload/ ${WORK}/actual/ || { cleanup ; exit 1 ; }

# verify that downloaded files match source files
for i in 1 2 3 4 ; do
	if gpg --status-fd 1 --output ${WORK}/actual/plaintext/data_$i --decrypt ${WORK}/actual/data_$i | grep -F -q '[GNUPG:] VALIDSIG' ; then
		echo "PASS: good signature on data_$i"
	else
		echo "FAIL: bad signature on data_$i"
		cleanup
		exit 1
	fi

	if diff -q ${WORK}/original/data_$i ${WORK}/actual/plaintext/data_$i ; then
		echo "PASS: correct data in data_$i"
	else
		echo "FAIL: downloaded file does not match original"
		cleanup
		exit 1
	fi
done

# verify that source files were deleted from the log directory
for i in 1 2 3 4 ; do
	if [ -e ${WORK}/logs/data_$i ] ; then
		echo "FAIL: failed to remove logs/data_$i"
		cleanup
		exit 1
	else
		echo "PASS: removed logs/data_$i"
	fi
done

cleanup
exit 0

