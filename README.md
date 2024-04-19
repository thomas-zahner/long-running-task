# long-running-task

A simple library to handle [long-running tasks](https://restfulapi.net/rest-api-design-for-long-running-tasks/).

## Why?

When exposing RESTful HTTP API paths to clients which trigger slow and/or computationally expensive
operations it is best practice to make use of the long-running task concept.

When making use of this concept a single endpoint, which blocks until the operation is completed,
is replaced with two endpoints. The first endpoint is used start the task.
The second endpoint is used to get the state of the running task.
It may be in progress (`Pending`) or may be completed (`Done`).

This crate helps to manage and keep track of long-running tasks.

## Features

...

## Get started

...
