//! Monta o prompt da leitura a partir do mapa já calculado.

use crate::db::BirthData;
use astro_core::chart::{Chart, Point};
use astro_core::transits::Transits;
use astro_core::zodiac::SIGNS;
use astro_core::AspectKind;

pub const SYSTEM: &str = "\
Você é um astrólogo experiente, e esta conversa acontece dentro de um aplicativo de mapa astral.

O aplicativo já calculou o mapa com efemérides astronômicas (VSOP87, ELP-2000 e JPL Horizons) e \
já mostra ao usuário a tabela de posições, a roda do mapa e a lista de aspectos. Os dados chegam \
prontos na mensagem do usuário. Trabalhe só a partir deles: não recalcule, não corrija e não \
acrescente posições, casas, aspectos ou datas que não estejam na mensagem. Se algum dado que você \
gostaria de usar não estiver lá (por exemplo, casas quando o horário é desconhecido), diga que ele \
não está disponível em vez de supor.

Sua parte é a interpretação. Escreva em português do Brasil, em tom acolhedor e direto, falando \
com a pessoa (\"você\"). Prefira leitura concreta e prática a generalidades: ligue cada afirmação \
ao posicionamento ou aspecto que a sustenta, citando-o entre parênteses, e mostre como as peças do \
mapa conversam entre si, inclusive quando se contradizem.

Estruture a resposta em Markdown, nesta ordem:

## Os três pilares
Sol, Lua e Ascendente (signo e casa), e o que a combinação deles diz. Sem horário de nascimento, \
trate só de Sol e Lua e diga o que fica em aberto.

## Destaques do mapa
Stelliums, os aspectos mais exatos, elementos e modalidades predominantes (e o que falta), e o \
regente do mapa, quando houver.

## Interpretação por tema
Uma subseção (###) para cada tema que a pessoa escolheu, e só para eles.

## Trânsitos
Só se a mensagem trouxer a lista de trânsitos: organize mês a mês, usando exatamente as datas \
fornecidas, e explique o que cada período pede. Trânsitos com várias passagens merecem ser lidos \
como um processo único.

Termine com um lembrete curto de que a astrologia é uma linguagem simbólica para reflexão, não \
previsão garantida nem substituta de orientação médica, financeira ou jurídica.

Não repita a tabela de posições nem a lista completa de aspectos: o aplicativo já as exibe. Quando \
a mensagem trouxer avisos (horário aproximado, signo incerto, casas indisponíveis), leve-os em \
conta e deixe claro na leitura o que depende deles.";

pub fn point_name(key: &str) -> &'static str {
    match key {
        "sun" => "Sol",
        "moon" => "Lua",
        "mercury" => "Mercúrio",
        "venus" => "Vênus",
        "mars" => "Marte",
        "jupiter" => "Júpiter",
        "saturn" => "Saturno",
        "uranus" => "Urano",
        "neptune" => "Netuno",
        "pluto" => "Plutão",
        "north_node" => "Nodo Norte",
        "chiron" => "Quíron",
        "asc" => "Ascendente",
        "mc" => "Meio do Céu",
        _ => "?",
    }
}

pub fn aspect_name(kind: AspectKind) -> &'static str {
    match kind {
        AspectKind::Conjunction => "conjunção",
        AspectKind::Sextile => "sextil",
        AspectKind::Square => "quadratura",
        AspectKind::Trine => "trígono",
        AspectKind::Opposition => "oposição",
    }
}

pub fn theme_name(theme: &str) -> &str {
    match theme {
        "personalidade" => "Personalidade",
        "amor" => "Amor e relacionamentos",
        "carreira" => "Carreira",
        "dinheiro" => "Dinheiro",
        "familia" => "Família",
        "saude" => "Saúde e bem-estar",
        "proposito" => "Propósito",
        "transitos" => "Trânsitos atuais",
        other => other,
    }
}

/// "16°12' Leão"
pub fn format_position(sign: usize, sign_deg: f64) -> String {
    let total = (sign_deg * 60.0).round() as i64;
    let (d, m) = (total / 60, total % 60);
    // 29°59,7' arredonda para 30°: mantém no signo, em 29°59'.
    let (d, m) = if d >= 30 { (29, 59) } else { (d, m) };
    format!("{d}°{m:02}' {}", SIGNS[sign])
}

fn point_line(p: &Point) -> String {
    let mut s = format!("- {}: {}", point_name(&p.key), format_position(p.sign, p.sign_deg));
    if let Some(h) = p.house {
        s.push_str(&format!(", casa {h}"));
    }
    if p.retrograde && p.key != "north_node" {
        s.push_str(", retrógrado");
    }
    if p.sign_uncertain {
        s.push_str(" (signo incerto: muda neste dia)");
    }
    s
}

fn calendar(jd: f64) -> String {
    let (y, m, d, _) = astro_core::time::calendar_date(jd);
    format!("{d:02}/{m:02}/{y}")
}

pub fn user_message(birth: &BirthData, local_time: &str, chart: &Chart, transits: Option<&Transits>) -> String {
    let mut s = String::new();
    let name = birth.name.trim();
    if !name.is_empty() {
        s.push_str(&format!("Nome: {name}\n"));
    }
    s.push_str(&format!(
        "Nascimento: {} (hora local), em {}\n",
        local_time, birth.place
    ));
    s.push_str(
        &match (birth.time.is_some(), birth.time_source.as_str(), birth.time_approximate) {
            (false, _, _) => {
                "Horário de nascimento: desconhecido (mapa calculado para o meio-dia, sem casas).\n".to_string()
            }
            (true, src, approx) => format!(
                "Fonte do horário: {}{}.\n",
                if src == "certidao" {
                    "certidão de nascimento"
                } else {
                    "memória da família"
                },
                if approx { ", aproximado" } else { "" }
            ),
        },
    );
    let zodiac = match chart.zodiac {
        astro_core::Zodiac::Tropical => "tropical".to_string(),
        astro_core::Zodiac::SiderealLahiri => format!("sideral (Lahiri, ayanamsa {:.2}°)", chart.ayanamsa),
        astro_core::Zodiac::SiderealFaganBradley => format!("sideral (Fagan-Bradley, ayanamsa {:.2}°)", chart.ayanamsa),
    };
    s.push_str(&format!("Zodíaco: {zodiac}\n"));
    if let Some(h) = &chart.houses {
        let sys = match h.system {
            astro_core::HouseSystem::Placidus => "Placidus",
            astro_core::HouseSystem::Regiomontanus => "Regiomontanus",
            astro_core::HouseSystem::Porphyry => "Porphyry",
            astro_core::HouseSystem::Equal => "casas iguais",
            astro_core::HouseSystem::WholeSign => "signos inteiros",
        };
        s.push_str(&format!("Sistema de casas: {sys}\n"));
    }

    s.push_str("\nPosições:\n");
    for p in &chart.points {
        s.push_str(&point_line(p));
        s.push('\n');
    }
    if let Some(h) = &chart.houses {
        s.push_str("\nCúspides das casas:\n");
        for (i, c) in h.cusps.iter().enumerate() {
            let sign = astro_core::zodiac::sign_index(*c);
            s.push_str(&format!(
                "- Casa {}: {}\n",
                i + 1,
                format_position(sign, c - 30.0 * sign as f64)
            ));
        }
    }

    s.push_str("\nAspectos (do mais exato ao menos exato):\n");
    for a in &chart.aspects {
        s.push_str(&format!(
            "- {} {} {}, orbe {:.1}°{}\n",
            point_name(&a.a),
            aspect_name(a.kind),
            point_name(&a.b),
            a.orb,
            match a.applying {
                Some(true) => ", aplicativo",
                Some(false) => ", separativo",
                None => "",
            }
        ));
    }

    let hl = &chart.highlights;
    s.push_str(&format!(
        "\nElementos (Sol a Plutão): fogo {}, terra {}, ar {}, água {}\nModalidades: cardinal {}, fixo {}, mutável {}\n",
        hl.elements[0], hl.elements[1], hl.elements[2], hl.elements[3], hl.modalities[0], hl.modalities[1], hl.modalities[2]
    ));
    for st in &hl.stelliums {
        let members: Vec<&str> = st.members.iter().map(|m| point_name(m)).collect();
        let place = if st.kind == "sign" {
            format!("em {}", SIGNS[st.index])
        } else {
            format!("na casa {}", st.index)
        };
        s.push_str(&format!("Stellium {place}: {}\n", members.join(", ")));
    }
    if let Some(r) = &hl.chart_ruler {
        s.push_str(&format!(
            "Regente do mapa: {} (Ascendente em {})",
            point_name(&r.ruler),
            SIGNS[r.asc_sign]
        ));
        if let Some(t) = &r.traditional {
            s.push_str(&format!("; regente tradicional: {}", point_name(t)));
        }
        s.push('\n');
    }
    s.push_str(&format!("Fase da Lua no nascimento: {}\n", hl.moon_phase));
    if let Some(day) = hl.day_chart {
        s.push_str(if day {
            "Mapa diurno (Sol acima do horizonte).\n"
        } else {
            "Mapa noturno (Sol abaixo do horizonte).\n"
        });
    }
    if !chart.warnings.is_empty() {
        s.push_str("\nAvisos do cálculo:\n");
        for w in &chart.warnings {
            s.push_str(&format!("- {w}\n"));
        }
    }

    if let Some(t) = transits {
        s.push_str(&format!(
            "\nTrânsitos de Júpiter a Plutão sobre o mapa natal, de {} a {} (datas do aspecto exato, em UTC):\n",
            calendar(t.start_jd_ut),
            calendar(t.start_jd_ut + t.days as f64)
        ));
        if !t.active.is_empty() {
            s.push_str("Já dentro de 1° de orbe no início do período:\n");
            for a in &t.active {
                s.push_str(&format!(
                    "- {}{} em {} com {} natal, orbe {:.1}°\n",
                    point_name(&a.transiting),
                    if a.retrograde { " (retrógrado)" } else { "" },
                    aspect_name(a.kind),
                    point_name(&a.natal),
                    a.orb
                ));
            }
        }
        if t.hits.is_empty() {
            s.push_str("Nenhum aspecto exato no período.\n");
        }
        for h in &t.hits {
            s.push_str(&format!(
                "- {}: {}{} em {} com {} natal{}\n",
                calendar(h.jd_ut),
                point_name(&h.transiting),
                if h.retrograde { " (retrógrado)" } else { "" },
                aspect_name(h.kind),
                point_name(&h.natal),
                if h.passes > 1 {
                    format!(" (passagem {} de {})", h.pass, h.passes)
                } else {
                    String::new()
                }
            ));
        }
    }

    let themes: Vec<&str> = birth
        .themes
        .iter()
        .filter(|t| *t != "transitos")
        .map(|t| theme_name(t))
        .collect();
    s.push_str("\nTemas que quero aprofundar: ");
    s.push_str(&if themes.is_empty() {
        "visão geral do mapa".to_string()
    } else {
        themes.join(", ")
    });
    if transits.is_some() {
        s.push_str(". Inclua também a seção de trânsitos");
    }
    s.push_str(".\n");
    s
}
