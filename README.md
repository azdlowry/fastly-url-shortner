# Fastly URL Shortener

A simple, serverless URL shortener running on [Fastly Compute@Edge](https://www.fastly.com/products/compute-at-edge).

## Features

- Shorten long URLs to easy-to-share links
- Fast, global edge delivery
- No backend server required

## Getting Started

### Prerequisites

- [Fastly account](https://developer.fastly.com/)
- [Fastly CLI](https://developer.fastly.com/reference/cli/)
- [Rust toolchain](https://www.rust-lang.org/tools/install) (if building from source)

### Deploy

1. Clone this repository:
    ```sh
    git clone https://github.com/yourusername/fastly-url-shortener.git
    cd fastly-url-shortener
    ```

2. Build and deploy to Fastly:
    ```sh
    fastly compute deploy
    ```

3. Follow the CLI prompts to complete deployment.

### Unit Testing

- Install bininstall
- Install nextest
- Install viceroy
- Run tests:
    ```sh
    cargo nextest run
    ```

## Configuration

- Update environment variables or Fastly service settings as needed.

## License

MIT License

Copyright (c) 2025 Andy Lowry

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
