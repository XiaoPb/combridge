local json = {}

local function skip_whitespace(str, pos)
    while pos <= #str do
        local c = string.sub(str, pos, pos)
        if c == ' ' or c == '\t' or c == '\n' or c == '\r' then
            pos = pos + 1
        else
            break
        end
    end
    return pos
end

local function parse_value(str, pos)
    pos = skip_whitespace(str, pos)
    
    if pos > #str then
        return nil, pos
    end
    
    local c = string.sub(str, pos, pos)
    
    if c == '{' then
        return parse_object(str, pos)
    elseif c == '[' then
        return parse_array(str, pos)
    elseif c == '"' then
        return parse_string(str, pos)
    elseif c == 't' then
        if string.sub(str, pos, pos + 3) == 'true' then
            return true, pos + 4
        end
    elseif c == 'f' then
        if string.sub(str, pos, pos + 4) == 'false' then
            return false, pos + 5
        end
    elseif c == 'n' then
        if string.sub(str, pos, pos + 3) == 'null' then
            return nil, pos + 4
        end
    elseif c == '-' or (c >= '0' and c <= '9') then
        return parse_number(str, pos)
    end
    
    return nil, pos
end

local function parse_string(str, pos)
    if string.sub(str, pos, pos) ~= '"' then
        return nil, pos
    end
    
    pos = pos + 1
    local result = {}
    local i = pos
    
    while i <= #str do
        local c = string.sub(str, i, i)
        
        if c == '"' then
            return table.concat(result), i + 1
        elseif c == '\\' then
            i = i + 1
            local escape = string.sub(str, i, i)
            if escape == 'n' then
                table.insert(result, '\n')
            elseif escape == 't' then
                table.insert(result, '\t')
            elseif escape == 'r' then
                table.insert(result, '\r')
            elseif escape == '"' then
                table.insert(result, '"')
            elseif escape == '\\' then
                table.insert(result, '\\')
            else
                table.insert(result, escape)
            end
        else
            table.insert(result, c)
        end
        i = i + 1
    end
    
    return nil, pos
end

local function parse_number(str, pos)
    local i = pos
    local c = string.sub(str, i, i)
    
    if c == '-' then
        i = i + 1
    end
    
    while i <= #str do
        c = string.sub(str, i, i)
        if c >= '0' and c <= '9' then
            i = i + 1
        else
            break
        end
    end
    
    if c == '.' then
        i = i + 1
        while i <= #str do
            c = string.sub(str, i, i)
            if c >= '0' and c <= '9' then
                i = i + 1
            else
                break
            end
        end
    end
    
    if c == 'e' or c == 'E' then
        i = i + 1
        c = string.sub(str, i, i)
        if c == '+' or c == '-' then
            i = i + 1
        end
        while i <= #str do
            c = string.sub(str, i, i)
            if c >= '0' and c <= '9' then
                i = i + 1
            else
                break
            end
        end
    end
    
    local num_str = string.sub(str, pos, i - 1)
    return tonumber(num_str), i
end

local function parse_array(str, pos)
    if string.sub(str, pos, pos) ~= '[' then
        return nil, pos
    end
    
    pos = pos + 1
    local result = {}
    
    pos = skip_whitespace(str, pos)
    
    if string.sub(str, pos, pos) == ']' then
        return result, pos + 1
    end
    
    while true do
        local value
        value, pos = parse_value(str, pos)
        table.insert(result, value)
        
        pos = skip_whitespace(str, pos)
        local c = string.sub(str, pos, pos)
        
        if c == ']' then
            return result, pos + 1
        elseif c == ',' then
            pos = pos + 1
        else
            return nil, pos
        end
    end
end

local function parse_object(str, pos)
    if string.sub(str, pos, pos) ~= '{' then
        return nil, pos
    end
    
    pos = pos + 1
    local result = {}
    
    pos = skip_whitespace(str, pos)
    
    if string.sub(str, pos, pos) == '}' then
        return result, pos + 1
    end
    
    while true do
        pos = skip_whitespace(str, pos)
        
        local key
        key, pos = parse_string(str, pos)
        
        pos = skip_whitespace(str, pos)
        
        if string.sub(str, pos, pos) ~= ':' then
            return nil, pos
        end
        pos = pos + 1
        
        local value
        value, pos = parse_value(str, pos)
        
        result[key] = value
        
        pos = skip_whitespace(str, pos)
        local c = string.sub(str, pos, pos)
        
        if c == '}' then
            return result, pos + 1
        elseif c == ',' then
            pos = pos + 1
        else
            return nil, pos
        end
    end
end

function json.decode(str)
    local result, pos = parse_value(str, 1)
    return result
end

local function encode_value(value)
    if type(value) == 'nil' then
        return 'null'
    elseif type(value) == 'boolean' then
        return value and 'true' or 'false'
    elseif type(value) == 'number' then
        return tostring(value)
    elseif type(value) == 'string' then
        local result = {'"'}
        for i = 1, #value do
            local c = string.sub(value, i, i)
            if c == '"' then
                table.insert(result, '\\"')
            elseif c == '\\' then
                table.insert(result, '\\\\')
            elseif c == '\n' then
                table.insert(result, '\\n')
            elseif c == '\t' then
                table.insert(result, '\\t')
            elseif c == '\r' then
                table.insert(result, '\\r')
            else
                table.insert(result, c)
            end
        end
        table.insert(result, '"')
        return table.concat(result)
    elseif type(value) == 'table' then
        local is_array = true
        local count = 0
        for _ in pairs(value) do
            count = count + 1
        end
        for i = 1, count do
            if value[i] == nil then
                is_array = false
                break
            end
        end
        
        if is_array then
            local parts = {}
            for i, v in ipairs(value) do
                table.insert(parts, encode_value(v))
            end
            return '[' .. table.concat(parts, ',') .. ']'
        else
            local parts = {}
            for k, v in pairs(value) do
                table.insert(parts, encode_value(tostring(k)) .. ':' .. encode_value(v))
            end
            return '{' .. table.concat(parts, ',') .. '}'
        end
    else
        return 'null'
    end
end

function json.encode(value)
    return encode_value(value)
end

return json
