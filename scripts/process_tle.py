from astropy import units as u
from skyfield.api import load, EarthSatellite


def process_tle(name, line1, line2, ts):
    try:
        # Create EarthSatellite from TLE using skyfield
        satellite = EarthSatellite(line1, line2, name, ts)

        # Get position and velocity at epoch (GCRS frame)
        # skyfield uses its own time objects
        t = satellite.epoch
        geocentric = satellite.at(t)

        # Extract position and velocity vectors in km and km/s
        pos = geocentric.position.km
        vel = geocentric.velocity.km_per_s
        # Construct dictionary
        return {
            "name": name,
            "px":pos[0],
            "py":pos[1],
            "pz":pos[2],
            "vx":vel[0],
            "vy":vel[1],
            "vz":vel[2],
        }
    except Exception as e:
        print(f"Error processing {name}: {e}")
        return None


def main():
    input_file = "res/beidou.txt"
    output_file = "res/beidou.rs"

    data = []

    print(f"Reading TLEs from {input_file}...")

    # Load timescale for skyfield
    ts = load.timescale()

    with open(input_file, "r") as f:
        lines = [line.strip() for line in f if line.strip()]

    count = 0
    i = 0
    while i < len(lines):
        if i + 2 >= len(lines):
            break

        name = lines[i]
        line1 = lines[i + 1]
        line2 = lines[i + 2]

        if not line1.startswith("1 ") or not line2.startswith("2 "):
            print(f"Skipping malformed group at line {i + 1}: {name}")
            i += 1
            continue

        element = process_tle(name, line1, line2, ts)
        if element:
            data.append(element)
            count += 1

        i += 3

    print(f"Processed {count} satellites.")

    with open(output_file, "w") as f:
        f.write(f"pub static CLIENT_SATELLITES: [Satellite; {len(data)}] = [\n")
        for sat in data:
            f.write(f'    Satellite {{ name: "{sat["name"]}", px: {sat["px"]}, py: {sat["py"]}, pz: {sat["pz"]}, vx: {sat["vx"]}, vy: {sat["vy"]}, vz: {sat["vz"]} }},\n')
        f.write("];\n")

    print(f"Saved to {output_file}")


if __name__ == "__main__":
    main()
