from astropy import units as u
from poliastro.bodies import Earth
from poliastro.twobody import Orbit
from skyfield.api import load, EarthSatellite

from scripts.gen_by_fly import generate_all_candidates


def process_tle(name, line1, line2, ts):
    try:
        # Create EarthSatellite from TLE using skyfield
        satellite = EarthSatellite(line1, line2, name, ts)

        # Get position and velocity at epoch (GCRS frame)
        t = satellite.epoch
        geocentric = satellite.at(t)

        # Extract position and velocity vectors in km and km/s
        pos = geocentric.position.km
        vel = geocentric.velocity.km_per_s

        # Create poliastro Orbit
        r_vec = pos * u.km
        v_vec = vel * u.km / u.s
        orbit = Orbit.from_vectors(Earth, r_vec, v_vec)

        # Generate candidates
        by_fly = generate_all_candidates(orbit)

        # Construct dictionary
        return {
            "name": name,
            "px": pos[0],
            "py": pos[1],
            "pz": pos[2],
            "vx": vel[0],
            "vy": vel[1],
            "vz": vel[2],
            "by_fly_orbits": by_fly
        }
    except Exception as e:
        import traceback
        traceback.print_exc()
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
            f.write(f'    Satellite {{\n')
            f.write(f'        name: "{sat["name"]}",\n')
            f.write(f'        r_km: [{sat["px"]}, {sat["py"]}, {sat["pz"]}],\n')
            f.write(f'        v_km_s: [{sat["vx"]}, {sat["vy"]}, {sat["vz"]}],\n')
            f.write(f'        by_fly_orbits: &[\n')

            for orbit in sat["by_fly_orbits"]:
                f.write(f'            OrbitParam {{\n')
                f.write(f'                orbit_type: "{orbit["type"]}",\n')
                f.write(f'                params: "{orbit["params"]}",\n')
                f.write(f'                a: {orbit["a"]},\n')
                f.write(f'                ecc: {orbit["ecc"]},\n')
                f.write(f'                inc: {orbit["inc"]},\n')
                f.write(f'                raan: {orbit["raan"]},\n')
                f.write(f'                argp: {orbit["argp"]},\n')
                f.write(f'                nu: {orbit["nu"]},\n')
                f.write(f'                m: {orbit["M"]},\n')
                f.write(f'            }},\n')

            f.write(f'        ],\n')
            f.write(f'    }},\n')
        f.write("];\n")

    print(f"Saved to {output_file}")


if __name__ == "__main__":
    main()
