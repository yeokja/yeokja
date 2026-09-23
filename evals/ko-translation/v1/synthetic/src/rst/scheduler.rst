Work-Stealing Scheduler
=======================

Each worker thread owns a double-ended queue of tasks. It pushes and pops at the
back of its own queue, while idle workers steal from the front of other queues.
This keeps recently created tasks, which are likely to share data in the cache,
on the thread that created them.

The scheduler exposes a single entry point, :func:`spawn`, which accepts a closure
and returns a :class:`JoinHandle`. Calling ``JoinHandle.join()`` from inside a task
does not block the worker; instead, it runs other pending tasks until the awaited
one completes.

Steal attempts pick a victim uniformly at random. Deterministic victim selection
was tried first, but it caused *convoys* in which every idle worker targeted the
same busy queue at once. See the :term:`victim selection` entry in the glossary and the
`benchmark results`_ for the numbers.

.. warning::

   Tasks must not hold a lock across a call to ``join()``. The worker may run an
   unrelated task that tries to take the same lock, and the program will
   deadlock.

.. _benchmark results: https://example.org/sched/bench
