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

- Manage and keep track of the progress of tasks and their results
- Enable the `serde` feature to make tasks (de-)serialisable and use them in combination with a web-frameworks
- Configure a lifespan to prevent completed tasks from living on indefinitely if they are not retrieved

## Get started

Take a look at [the Rocket example](examples/rocket/src/main.rs) to learn how to design REST API endpoints for long-running tasks.
