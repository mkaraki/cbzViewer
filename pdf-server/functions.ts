import * as Sentry from "@sentry/bun";
import * as fs from "node:fs";
import * as path from "node:path";

const getCanonicalCbzDir = (): string|false => {
    let cbzDir = Bun.env.CBZ_DIR;
    if (cbzDir === undefined) {
        Sentry.logger.fatal("Environment variable: CBZ_DIR is undefined");
        console.error("Environment variable: CBZ_DIR is undefined");
        process.exit(1);
    }

    try {
        cbzDir = fs.realpathSync(cbzDir);
    } catch (error) {
        Sentry.captureException(error);
        console.error(error);
        return false;
    }

    if (!cbzDir.endsWith("/") && !cbzDir.endsWith("\\")) {
        cbzDir += path.sep;
    }

    return cbzDir;
}

const getRealPath = (virtual_path: string): string|false => {
    let realCbzDir = getCanonicalCbzDir();
    if (realCbzDir === false) {
        return false;
    }

    let v_path = virtual_path.replace(/\\/g, "/");
    if (virtual_path.startsWith("/")) {
        v_path = v_path.substring(1);
    }

    let realPath = realCbzDir + v_path;

    try {
        realPath = fs.realpathSync(realPath);
    } catch (error) {
        return false;
    }

    if (!isSafePath(realPath)) {
        return false;
    }
    return realPath;
}

const getVirtualPath = (real_path: string): string|false => {
    let realCbzDir = getCanonicalCbzDir();
    if (realCbzDir === false) {
        return false;
    }

    if (!isSafePath(real_path)) {
        return false;
    }

    let stat;
    try {
        stat = fs.statSync(real_path);
    } catch (error) {
        return false;
    }

    if (stat.isDirectory() && (!real_path.endsWith("/") && !real_path.endsWith("\\"))) {
        real_path += path.sep;
    }

    const virtualPath = real_path.substring(realCbzDir.length);
    return virtualPath;
};

const isSafePath = (real_path: string): boolean => {
    const canonicalCbzDir = getCanonicalCbzDir();
    if (canonicalCbzDir === false) {
        return false;
    }

    if (!real_path.endsWith("/") && !real_path.endsWith("\\")) {
        real_path += path.sep;
    }

    return real_path.startsWith(canonicalCbzDir);
}

const getParentIfExists = (virtual_path: string): string|false => {
    const realPath = getRealPath(virtual_path);
    if (realPath === false) {
        return false;
    }
    let virtualPath = getVirtualPath(realPath);
    if (virtualPath === false) {
        return false;
    }
    
    virtualPath = virtualPath.trim().replace(/\\/g, "/");
    
    let splitPath = virtualPath.split("/");
    splitPath = splitPath.filter(v => v !== '');
    
    if (splitPath.length == 0) {
        // No parent dir
        return false;
    }
    
    if (splitPath.length == 1) {
        return '';
    }
    const parentPath = splitPath.join('/');
    return parentPath;
}

const getThumbSize = 
    (original_width: number,  original_height: number): {new_width: number, new_height: number} => 
{
    const MAX_SIZE = 160;
    
    if (original_width > original_height) {
        if (original_width < MAX_SIZE) {
            return {
                new_width: original_width,
                new_height: original_height,
            };
        }
        
        const new_width = MAX_SIZE;
        const ratio = new_width / original_width;
        const new_height = original_height * ratio;
        
        return {new_width, new_height};
    } else {
        if (original_height < MAX_SIZE) {
            return {
                new_width: original_width,
                new_height: original_height,
            };
        }
        
        const new_height = MAX_SIZE;
        const ratio = new_height / original_height;
        const new_width = original_width * ratio;
        
        return {new_width, new_height};
    }
}

const getThumbScale =
    (original_width: number,  original_height: number): number =>
{
    const MAX_SIZE = 160;
    const MAX_SIZE_F = 160.0;

    if (original_width > original_height) {
        if (original_width < MAX_SIZE) {
            return 1.0;
        }

        return MAX_SIZE / original_width;
    } else {
        if (original_height < MAX_SIZE) {
            return 1.0;
        }

        return MAX_SIZE / original_height;
    }
}

export {
    getCanonicalCbzDir,
    getRealPath,
    getVirtualPath,
    isSafePath,
    getParentIfExists,
    getThumbSize,
    getThumbScale,
}