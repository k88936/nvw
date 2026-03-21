import numpy as np
from astropy import units as u
from poliastro.bodies import Earth
from poliastro.twobody import Orbit


def lvlh_to_inertial(r_t, v_t, rho, drho):
    """
    将 LVLH 系中的相对位置速度转换为惯性系绝对位置速度。
    参数:
        r_t, v_t : 目标卫星在惯性系中的位置速度矢量 (3,)，单位需一致（如 km, km/s）
        rho, drho: 服务卫星在 LVLH 系中的相对位置速度 (3,)，单位 km, km/s
    返回:
        r_s, v_s : 服务卫星在惯性系中的位置速度矢量
    """
    # Use numpy norm since inputs are values
    norm = np.linalg.norm

    # 构建 LVLH 坐标系基向量
    x_hat = r_t / norm(r_t)
    z_hat = np.cross(r_t, v_t) / norm(np.cross(r_t, v_t))
    y_hat = np.cross(z_hat, x_hat)
    # 旋转矩阵（从 LVLH 到惯性系，列向量为基）
    R = np.vstack([x_hat, y_hat, z_hat]).T

    # 目标角速度大小
    omega = norm(np.cross(r_t, v_t)) / norm(r_t) ** 2
    omega_vec = omega * z_hat

    # 转换
    r_s = r_t + R @ rho
    v_s = v_t + R @ (drho + np.cross(omega_vec, rho))
    return r_s, v_s


def mean_to_true_anomaly(M, ecc):
    """
    通过开普勒方程从平近点角 M 计算真近点角 nu。
    参数:
        M : 平近点角 (rad)
        ecc: 偏心率
    返回:
        nu: 真近点角 (rad)
    """
    if ecc < 1e-12:
        return M
    # 用牛顿法求解偏近点角 E
    E = M
    for _ in range(10):
        f = E - ecc * np.sin(E) - M
        f_prime = 1 - ecc * np.cos(E)
        delta = f / f_prime
        E -= delta
        if abs(delta) < 1e-12:
            break
    # 从偏近点角计算真近点角
    nu = 2 * np.arctan2(np.sqrt(1 + ecc) * np.sin(E / 2), np.sqrt(1 - ecc) * np.cos(E / 2))
    return nu


def true_to_mean_anomaly(nu, ecc):
    """
    从真近点角 nu 计算平近点角 M。
    """
    # 先算偏近点角 E
    E = 2 * np.arctan2(np.sqrt(1 - ecc) * np.sin(nu / 2), np.sqrt(1 + ecc) * np.cos(nu / 2))
    M = E - ecc * np.sin(E)
    return M


def generate_ellipse_candidates(target_orbit, b_km_list, phi_deg_list):
    """生成椭圆型伴飞候选"""
    mu = target_orbit.attractor.k.to(u.km ** 3 / u.s ** 2).value
    a = target_orbit.a.to(u.km).value
    omega = np.sqrt(mu / a ** 3)  # rad/s
    r_t, v_t = target_orbit.rv()
    r_t = r_t.to(u.km).value
    v_t = v_t.to(u.km / u.s).value

    candidates = []
    for b in b_km_list:
        for phi_deg in phi_deg_list:
            phi = np.deg2rad(phi_deg)
            # 初始相对速度（LVLH）
            dx0 = omega * b * np.cos(phi)
            dy0 = -2 * omega * b * np.sin(phi)
            dz0 = 0.0
            rho = np.array([0.0, 0.0, 0.0])
            drho = np.array([dx0, dy0, dz0])
            r_s, v_s = lvlh_to_inertial(r_t, v_t, rho, drho)
            # 从位置速度构建轨道
            orbit = Orbit.from_vectors(Earth, r_s * u.km, v_s * u.km / u.s)
            candidates.append({
                'type': 'ellipse',
                'params': f'b={b}km,phi={phi_deg}°',
                'a': orbit.a.to(u.km).value,
                'ecc': orbit.ecc.value,
                'inc': orbit.inc.to(u.deg).value,
                'raan': orbit.raan.to(u.deg).value,
                'argp': orbit.argp.to(u.deg).value,
                'nu': orbit.nu.to(u.deg).value,
                'M': true_to_mean_anomaly(orbit.nu.to(u.rad).value, orbit.ecc.value)
            })
    return candidates


def generate_oscillation_candidates(target_orbit, dz_km_list):
    """生成振荡型伴飞候选"""
    mu = target_orbit.attractor.k.to(u.km ** 3 / u.s ** 2).value
    a = target_orbit.a.to(u.km).value
    omega = np.sqrt(mu / a ** 3)
    r_t, v_t = target_orbit.rv()
    r_t = r_t.to(u.km).value
    v_t = v_t.to(u.km / u.s).value

    candidates = []
    for dz in dz_km_list:
        dx0 = 0.0
        dy0 = 0.0
        dz0 = omega * dz
        rho = np.array([0.0, 0.0, 0.0])
        drho = np.array([dx0, dy0, dz0])
        r_s, v_s = lvlh_to_inertial(r_t, v_t, rho, drho)
        orbit = Orbit.from_vectors(Earth, r_s * u.km, v_s * u.km / u.s)
        candidates.append({
            'type': 'oscillation',
            'params': f'dz={dz}km',
            'a': orbit.a.to(u.km).value,
            'ecc': orbit.ecc.value,
            'inc': orbit.inc.to(u.deg).value,
            'raan': orbit.raan.to(u.deg).value,
            'argp': orbit.argp.to(u.deg).value,
            'nu': orbit.nu.to(u.deg).value,
            'M': true_to_mean_anomaly(orbit.nu.to(u.rad).value, orbit.ecc.value)
        })
    return candidates


def generate_hover_candidates(target_orbit, h_km_list, x_max_ratio_list):
    """生成盘旋型拟伴飞候选（跳跃型轨迹）"""
    mu = target_orbit.attractor.k.to(u.km ** 3 / u.s ** 2).value
    a = target_orbit.a.to(u.km).value
    omega = np.sqrt(mu / a ** 3)
    r_t, v_t = target_orbit.rv()
    r_t = r_t.to(u.km).value
    v_t = v_t.to(u.km / u.s).value

    candidates = []
    for h in h_km_list:
        for ratio in x_max_ratio_list:
            # 期望的最大径向位移
            x_max = ratio * h
            # 根据跳跃型公式估算 dv_y
            dv_y = x_max * omega / 4.0
            rho = np.array([h, 0.0, 0.0])
            drho = np.array([0.0, dv_y, 0.0])
            r_s, v_s = lvlh_to_inertial(r_t, v_t, rho, drho)
            orbit = Orbit.from_vectors(Earth, r_s * u.km, v_s * u.km / u.s)
            candidates.append({
                'type': 'hover',
                'params': f'h={h}km,ratio={ratio}',
                'a': orbit.a.to(u.km).value,
                'ecc': orbit.ecc.value,
                'inc': orbit.inc.to(u.deg).value,
                'raan': orbit.raan.to(u.deg).value,
                'argp': orbit.argp.to(u.deg).value,
                'nu': orbit.nu.to(u.deg).value,
                'M': true_to_mean_anomaly(orbit.nu.to(u.rad).value, orbit.ecc.value)
            })
    return candidates


def generate_alldir_candidates(target_orbit, b_km_list, dz_km_list, psi_deg_list, phi_deg=0):
    """生成全向伴飞候选（椭圆+振荡合成）"""
    mu = target_orbit.attractor.k.to(u.km ** 3 / u.s ** 2).value
    a = target_orbit.a.to(u.km).value
    omega = np.sqrt(mu / a ** 3)
    r_t, v_t = target_orbit.rv()
    r_t = r_t.to(u.km).value
    v_t = v_t.to(u.km / u.s).value
    phi = np.deg2rad(phi_deg)

    candidates = []
    for b in b_km_list:
        for dz in dz_km_list:
            for psi_deg in psi_deg_list:
                psi = np.deg2rad(psi_deg)
                dx0 = omega * b * np.cos(phi) * np.cos(psi)
                dy0 = -2 * omega * b * np.sin(phi) * np.cos(psi)
                dz0 = omega * dz * np.sin(psi)
                rho = np.array([0.0, 0.0, 0.0])
                drho = np.array([dx0, dy0, dz0])
                r_s, v_s = lvlh_to_inertial(r_t, v_t, rho, drho)
                orbit = Orbit.from_vectors(Earth, r_s * u.km, v_s * u.km / u.s)
                candidates.append({
                    'type': 'all_dir',
                    'params': f'b={b}km,dz={dz}km,psi={psi_deg}°',
                    'a': orbit.a.to(u.km).value,
                    'ecc': orbit.ecc.value,
                    'inc': orbit.inc.to(u.deg).value,
                    'raan': orbit.raan.to(u.deg).value,
                    'argp': orbit.argp.to(u.deg).value,
                    'nu': orbit.nu.to(u.deg).value,
                    'M': true_to_mean_anomaly(orbit.nu.to(u.rad).value, orbit.ecc.value)
                })
    return candidates


def generate_sameorbit_candidates(target_orbit, delta_M_deg_list):
    """生成共轨伴飞候选（相同轨道，不同平近点角）"""
    # 获取目标轨道的六个根数
    a = target_orbit.a
    ecc = target_orbit.ecc
    inc = target_orbit.inc
    raan = target_orbit.raan
    argp = target_orbit.argp
    nu0 = target_orbit.nu
    M0 = true_to_mean_anomaly(nu0.to(u.rad).value, ecc.value)

    candidates = []
    for delta_M_deg in delta_M_deg_list:
        delta_M = np.deg2rad(delta_M_deg)
        M_new = M0 + delta_M
        # 归一化到 [0, 2π)
        M_new = M_new % (2 * np.pi)
        # 从平近点角求真近点角
        nu_new = mean_to_true_anomaly(M_new, ecc.value)
        # 构建新轨道
        orbit = Orbit.from_classical(Earth, a, ecc, inc, raan, argp, nu_new * u.rad)
        candidates.append({
            'type': 'same_orbit',
            'params': f'delta_M={delta_M_deg}°',
            'a': orbit.a.to(u.km).value,
            'ecc': orbit.ecc.value,
            'inc': orbit.inc.to(u.deg).value,
            'raan': orbit.raan.to(u.deg).value,
            'argp': orbit.argp.to(u.deg).value,
            'nu': orbit.nu.to(u.deg).value,
            'M': M_new
        })
    return candidates


def generate_all_candidates(target_orbit):
    # 定义各类型候选的参数离散集（可根据需要调整）
    # 椭圆型：短半轴 (km) 和相位角 (deg)
    b_vals = [0.5, 1, 2, 5]
    phi_vals = [0, 90, 180, 270]
    # 振荡型：面外振幅 (km)
    dz_vals = [1, 2, 5]
    # 盘旋型：初始高度 (km) 和最大径向位移倍数（相对高度）
    h_vals = [1, 2, 5]
    ratio_vals = [2, 3, 4]  # x_max = ratio * h
    # 全向型：组合 b, dz, 旋转角 psi
    psi_vals = [0, 30, 60, 90]
    # 共轨型：相位差 (deg)
    delta_m_vals = [0.1, 0.5, 1, 2, 5]

    candidates = []
    candidates.extend(generate_ellipse_candidates(target_orbit, b_vals, phi_vals))
    candidates.extend(generate_oscillation_candidates(target_orbit, dz_vals))
    candidates.extend(generate_hover_candidates(target_orbit, h_vals, ratio_vals))
    candidates.extend(generate_alldir_candidates(target_orbit, b_vals, dz_vals, psi_vals))
    candidates.extend(generate_sameorbit_candidates(target_orbit, delta_m_vals))
    return candidates
