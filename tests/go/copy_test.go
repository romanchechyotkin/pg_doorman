package doorman_test

import (
	"database/sql"
	"os"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func Test_Copy(t *testing.T) {
	db, errOpen := sql.Open("postgres", os.Getenv("DATABASE_URL"))
	assert.NoError(t, errOpen)
	defer db.Close()
	// prepare
	{
		_, errExec := db.Exec("drop table if exists test_copy; create table test_copy(t text);")
		assert.NoError(t, errExec)
	}
	// run tx with lock.
	{
		txLock, errTxLock := db.Begin()
		assert.NoError(t, errTxLock)
		defer txLock.Commit()
		_, errExec := txLock.Exec("lock table test_copy")
		assert.NoError(t, errExec)
	}
	// run with timeout
	{
		txCopy, errTxCopy := db.Begin()
		assert.NoError(t, errTxCopy)
		_, errExec := txCopy.Exec("set local statement_timeout to '1s'")
		assert.NoError(t, errExec)
		_, errExec = txCopy.Exec("COPY test_copy(t) FROM stdin")
		assert.Error(t, errExec)
		assert.NoError(t, txCopy.Rollback())
	}
	time.Sleep(time.Minute)
}
