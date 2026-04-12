-- IMU Parser Script
-- Parses IMU sensor data (accelerometer, gyroscope)

local parser = {}

parser.name = "IMU Parser"
parser.description = "Parse IMU sensor data (accelerometer, gyroscope)"
parser.author = "ComBridge"
parser.version = "1.0.0"

parser.fields = {
    { key = "acc_x", path = "acc_x", unit = "g" },
    { key = "acc_y", path = "acc_y", unit = "g" },
    { key = "acc_z", path = "acc_z", unit = "g" },
    { key = "gyro_x", path = "gyro_x", unit = "dps" },
    { key = "gyro_y", path = "gyro_y", unit = "dps" },
    { key = "gyro_z", path = "gyro_z", unit = "dps" },
    { key = "temperature", path = "temperature", unit = "°C" },
}

function parser.parse(data)
    local success, json_obj = pcall(json.decode, data)
    if not success or type(json_obj) ~= "table" then
        return nil
    end
    
    local result = {}
    
    result.acc_x = json_obj.acc_x or json_obj.accel_x or json_obj.ax
    result.acc_y = json_obj.acc_y or json_obj.accel_y or json_obj.ay
    result.acc_z = json_obj.acc_z or json_obj.accel_z or json_obj.az
    result.gyro_x = json_obj.gyro_x or json_obj.gx
    result.gyro_y = json_obj.gyro_y or json_obj.gy
    result.gyro_z = json_obj.gyro_z or json_obj.gz
    result.temperature = json_obj.temp or json_obj.temperature
    
    return result
end

function parser.validate(data)
    return data ~= nil and #data > 0
end

return parser
