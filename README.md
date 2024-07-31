# kubeport
Exposing external computers to Kubernetes/the World Wide Web

# How it Works
The server just forwards traffic from a local server to the cloud.

# Features
- Supports HTTP/1.1 traffic


## Testing
To test we use Hypercorn...
```
python -m hypercorn app --bind 127.0.0.1:4032
```