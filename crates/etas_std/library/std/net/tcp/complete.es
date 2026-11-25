module std.net.tcp;

public type Host = { private host: string };
public type Port = { private port: u16 };
public type NetworkError = { code: string, message: string };

public type TcpOptions = {
    timeout: Option<std.stream.Timeout>,
    nodelay: bool,
};

public let DefaultTcpOptions = TcpOptions { timeout= None, nodelay= true };

impl Host {
    public flow parse(text: string) -> Result<Host, NetworkError> ![] {
        if text == "" { return Err(Host.invalid()); }
        if std.text.starts_with(text, "[") {
            if !std.text.ends_with(text, "]") { return Err(Host.invalid()); }
            let inside = std.text.strip_suffix(std.text.strip_prefix(text, "[").unwrap(), "]").unwrap();
            return match Host.ipv6(inside) {
                Some(value) => Ok(Host { host= value }),
                None => Err(Host.invalid()),
            };
        }
        let address = text;
        if std.text.contains(address, ":") {
            return match Host.ipv6(address) {
                Some(value) => Ok(Host { host= value }),
                None => Err(Host.invalid()),
            };
        }
        var numeric = true;
        for byte in address.utf8 {
            if (byte < 48 || byte > 57) && byte != 46 { numeric = false; }
        }
        if numeric {
            return match Host.ipv4(address) {
                Some(octets) => Ok(Host { host= Host.ipv4_text(octets) }),
                None => Err(Host.invalid()),
            };
        }
        let name = std.text.strip_suffix(address, ".").unwrap_or(address);
        if name == "" || name.length > 253 { return Err(Host.invalid()); }
        for label in std.text.split(name, ".") {
            let bytes = label.utf8;
            if bytes.length == 0 || bytes.length > 63 { return Err(Host.invalid()); }
            if bytes.get(0).unwrap() == 45 || bytes.get(bytes.length - 1).unwrap() == 45 {
                return Err(Host.invalid());
            }
            for byte in bytes {
                if !((byte >= 65 && byte <= 90) || (byte >= 97 && byte <= 122) ||
                     (byte >= 48 && byte <= 57) || byte == 45) {
                    return Err(Host.invalid());
                }
            }
        }
        return Ok(Host { host= std.text.lowercase(address) });
    }

    public flow to_string(self) -> string ![] {
        return self.host;
    }

    flow invalid() -> NetworkError ![] {
        return NetworkError { code= "invalid_input", message= "invalid hostname or IP address" };
    }

    flow ipv4(text: string) -> Option<Array<u16>> ![] {
        let parts = std.text.split(text, ".");
        if parts.length != 4 { return None; }
        var octets: Array<u16> = [];
        for part in parts {
            let bytes = part.utf8;
            if bytes.length == 0 || bytes.length > 3 { return None; }
            if bytes.length > 1 && bytes.get(0).unwrap() == 48 { return None; }
            var value: u16 = 0;
            for byte in bytes {
                if byte < 48 || byte > 57 { return None; }
                value = value * 10 + Host.digit(byte).unwrap();
            }
            if value > 255 { return None; }
            octets = octets.push(value);
        }
        return Some(octets);
    }

    flow ipv4_text(octets: Array<u16>) -> string ![] {
        var parts: Array<string> = [];
        for octet in octets { parts = parts.push(std.text.from_u16(octet)); }
        return std.text.join(parts, ".");
    }

    flow words(text: string, final: bool) -> Option<Array<u16>> ![] {
        if text == "" { return Some([]); }
        let parts = std.text.split(text, ":");
        var output: Array<u16> = [];
        var index: usize = 0;
        for part in parts {
            if std.text.contains(part, ".") {
                if !final || index + 1 != parts.length { return None; }
                let octets = match Host.ipv4(part) { Some(value) => value, None => { return None; }, };
                output = output.push(octets.get(0).unwrap() * 256 + octets.get(1).unwrap());
                output = output.push(octets.get(2).unwrap() * 256 + octets.get(3).unwrap());
            } else {
                let bytes = part.utf8;
                if bytes.length == 0 || bytes.length > 4 { return None; }
                var value: u16 = 0;
                for byte in bytes {
                    let digit = match Host.digit(byte) {
                        Some(value) => value,
                        None => { return None; },
                    };
                    value = value * 16 + digit;
                }
                output = output.push(value);
            }
            index = index + 1;
        }
        return Some(output);
    }

    flow ipv6(text: string) -> Option<string> ![] {
        let parts = std.text.split(text, "::");
        if parts.length > 2 { return None; }
        let compressed = parts.length == 2;
        let left = match Host.words(parts.get(0).unwrap(), !compressed) {
            Some(value) => value, None => { return None; },
        };
        let right = if compressed {
            match Host.words(parts.get(1).unwrap(), true) { Some(value) => value, None => { return None; }, }
        } else { [] };
        let supplied = left.length + right.length;
        if supplied > 8 || (!compressed && supplied != 8) || (compressed && supplied == 8) {
            return None;
        }
        var words = left;
        var missing = 8 - supplied;
        while missing > 0 {
            words = words.push(0);
            missing = missing - 1;
        }
        for word in right { words = words.push(word); }
        if words.get(0).unwrap() == 0 && words.get(1).unwrap() == 0 &&
           words.get(2).unwrap() == 0 && words.get(3).unwrap() == 0 &&
           words.get(4).unwrap() == 0 && words.get(5).unwrap() == 65535 {
            let a = words.get(6).unwrap();
            let b = words.get(7).unwrap();
            return Some("::ffff:" + Host.ipv4_text([a / 256, a % 256, b / 256, b % 256]));
        }
        var best_start: usize = 0;
        var best_length: usize = 1;
        var index: usize = 0;
        while index < 8 {
            if words.get(index).unwrap() == 0 {
                let start = index;
                while index < 8 && words.get(index).unwrap() == 0 { index = index + 1; }
                if index - start > best_length {
                    best_start = start;
                    best_length = index - start;
                }
            } else { index = index + 1; }
        }
        var result = "";
        index = 0;
        while index < 8 {
            if best_length > 1 && index == best_start {
                result = result + "::";
                index = index + best_length;
            } else {
                if result != "" && !std.text.ends_with(result, ":") { result = result + ":"; }
                result = result + Host.hex(words.get(index).unwrap());
                index = index + 1;
            }
        }
        return Some(result);
    }

    flow digit(byte: u8) -> Option<u16> ![] {
        return match byte {
            48 => Some(0),
            49 => Some(1),
            50 => Some(2),
            51 => Some(3),
            52 => Some(4),
            53 => Some(5),
            54 => Some(6),
            55 => Some(7),
            56 => Some(8),
            57 => Some(9),
            65 => Some(10),
            97 => Some(10),
            66 => Some(11),
            98 => Some(11),
            67 => Some(12),
            99 => Some(12),
            68 => Some(13),
            100 => Some(13),
            69 => Some(14),
            101 => Some(14),
            70 => Some(15),
            102 => Some(15),
            _ => None,
        };
    }

    flow hex(word: u16) -> string ![] {
        if word == 0 { return "0"; }
        let digits = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "a", "b", "c", "d", "e", "f"];
        var value = word;
        var result = "";
        while value != 0 {
            result = digits.get(value % 16).unwrap() + result;
            value = value / 16;
        }
        return result;
    }
}

impl Port {
    public flow new(value: u16) -> Result<Port, NetworkError> ![] {
        if value == 0 {
            return Err(NetworkError { code= "invalid_input", message= "connect port must be nonzero" });
        }
        return Ok(Port { port= value });
    }

    public flow value(self) -> u16 ![] {
        return self.port;
    }
}

// Only the host connection primitive creates a live connection handle.
type TcpConnection;
public type TcpStream = { private connection: TcpConnection };

public flow connect(host: Host, port: Port, options: TcpOptions = DefaultTcpOptions) -> TcpStream ![Error<NetworkError>] {
    return perform std.effects.Net.tcp_connect(host, port, options);
}

impl TcpStream ~ std.stream.ByteStream {}
