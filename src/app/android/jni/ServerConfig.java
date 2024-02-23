package info.scoria;

public class ServerConfig {
    public int port = 0;
    public long scope = 0; // use `String s = Long.toUnsignedString(x)`
    public ServerConfig(int port, long scope) {
        this.port = port;
        this.scope = scope;
    }
}
