package progress

import (
	"fmt"
	"io"
	"sync"
)

type Event struct {
	Name   string
	Status string
}

type Renderer struct {
	writer io.Writer
	mu     sync.Mutex
}

func NewRenderer(writer io.Writer) *Renderer {
	if writer == nil {
		writer = io.Discard
	}

	return &Renderer{writer: writer}
}

func (r *Renderer) Event(event Event) {
	r.EventName(event.Name, event.Status)
}

func (r *Renderer) EventName(name string, status string) {
	r.mu.Lock()
	defer r.mu.Unlock()

	// EventSink has no error return, so rendering is best-effort.
	fmt.Fprintf(r.writer, "%s: %s\n", name, status)
}
