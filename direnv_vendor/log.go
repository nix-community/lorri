package direnv_vendor

import "log"

func logDebug(format string, args ...interface{}) {
	log.Printf("lorri: debug: "+format, args...)
}
