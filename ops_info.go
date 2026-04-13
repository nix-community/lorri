package main

// op_info: print project info and daemon status.
// Mirrors src/ops.rs op_info().

import (
	"fmt"
)

// opInfo prints lorri project info to stdout.
// Exact output format mirrors ops.rs op_info().
func opInfo(paths *Paths, projectFile ProjectFile) error {
	nixFile := nixFilePathForProject(projectFile)
	gcRootPath := gcRootPathForProject(paths.GCRootDir, projectFile)

	// Try to connect to the daemon for DaemonInfo.
	daemonStatus := daemonInfoStatus(paths)

	// Check whether GC root exists.
	gcRootStr := ""
	if fileExists(gcRootPath.String()) {
		gcRootStr = gcRootPath.String()
	} else {
		gcRootStr = "GC roots do not exist. Has the project been built with lorri yet?"
	}

	fmt.Printf(`Project Shell File: %s
Project Garbage Collector Root: %s

General:
Lorri User GC Root Dir: %s
Lorri Daemon Socket: %s
Lorri Daemon Status: %s
`,
		nixFile,
		gcRootStr,
		paths.GCRootDir,
		paths.DaemonSocketFile,
		daemonStatus,
	)
	return nil
}

// daemonInfoStatus connects to the daemon and returns a human-readable status string.
// Mirrors the daemon_status computation in ops.rs op_info().
func daemonInfoStatus(paths *Paths) string {
	socketPath := NewSocketPath(paths.DaemonSocketFile)
	framing, err := connectClient(socketPath, CommDaemonInfo, defaultReadTimeout)
	if err != nil {
		return fmt.Sprintf("`lorri daemon` is not up: %v", err)
	}
	defer framing.Close()

	// Send empty DaemonInfo request, read empty reply.
	if err := framing.WriteMsg(defaultReadTimeout, struct{}{}); err != nil {
		return fmt.Sprintf("Problem connecting to the `lorri daemon`: %v", err)
	}
	var reply struct{}
	if err := framing.ReadMsg(defaultReadTimeout, &reply); err != nil {
		return fmt.Sprintf("Problem connecting to the `lorri daemon`: %v", err)
	}
	return "`lorri daemon` is running"
}
