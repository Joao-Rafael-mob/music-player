use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use std::{
    env,
    fs::{self, File},
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread::{self},
    time::Duration,
};
use trash;

fn main() -> io::Result<()> {
    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .expect("Could not get home directory");
    let mut music_dir = PathBuf::from(home_dir);
    music_dir.push("Músicas");
    let path = Path::new(&music_dir);

    let mut files = fs::read_dir(path)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;

    files = shuffle_files(files);

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Arc::new(Mutex::new(Sink::try_new(&stream_handle).unwrap()));
    let files = Arc::new(Mutex::new(files));
    let index = Arc::new(Mutex::new(0));

    let (tx, rx) = mpsc::channel::<String>();

    let sink_clone = Arc::clone(&sink);
    let files_clone = Arc::clone(&files);
    let index_clone = Arc::clone(&index);

    let playback_thread = thread::spawn(move || {
        playback_loop(rx, index_clone, files_clone, sink_clone);
    });

    let input_thread = thread::spawn(move || {
        input_lopp(tx);
    });

    playback_thread.join().unwrap();
    input_thread.join().unwrap();

    Ok(())
}

fn playback_loop(
    rx: mpsc::Receiver<String>,
    index: Arc<Mutex<usize>>,
    files: Arc<Mutex<Vec<PathBuf>>>,
    sink: Arc<Mutex<Sink>>,
) {
    loop {
        let mut idx = index.lock().unwrap();
        let mut file_list = files.lock().unwrap();

        if *idx >= file_list.len() {
            println!("Não há mais músicas na lista.");
            break;
        }

        play_music(&file_list[*idx], &mut sink.lock().unwrap()).unwrap();

        loop {
            if sink.lock().unwrap().empty() {
                *idx += 1;
                break;
            }

            if let Ok(command) = rx.try_recv() {
                match command.as_str() {
                    "next" => {
                        if *idx + 1 < file_list.len() {
                            *idx += 1;
                        } else {
                            println!("Já estamos na última música.");
                        }
                        break;
                    }
                    "prev" => {
                        if *idx > 0 {
                            *idx -= 1;
                        } else {
                            println!("Já estamos na primeira música.");
                        }
                        break;
                    }
                    "play" => {
                        sink.lock().unwrap().play();
                        println!("musica tocando");
                    }
                    "pause" => {
                        sink.lock().unwrap().pause();
                        println!("musica pausada")
                    }
                    "delete" => {
                        let file_to_delete = &file_list[*idx];
                        let _ = trash::delete(file_to_delete);
                        println!("Enviado para a lixeira: {:?}", file_to_delete);

                        file_list.remove(*idx);
                        if *idx >= file_list.len() && *idx > 0 {
                            *idx -= 1;
                        }
                        break;
                    }
                    "quit" => {
                        println!("Saindo do programa.");
                        return;
                    }
                    _ => println!("Comando inválido."),
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    }
}

fn input_lopp(tx: mpsc::Sender<String>) {
    loop {
        let user_input = get_user_input();
        let command = match user_input.as_str() {
            "D" => "next",
            "A" => "prev",
            "W" => "play",
            "S" => "pause",
            "L" => "delete",
            "Q" => "quit",
            _ => continue,
        };
        tx.send(command.to_string()).unwrap();
        if command == "quit" {
            break;
        }
    }
}

fn play_music(file_path: &PathBuf, sink: &mut Sink) -> io::Result<()> {
    sink.stop();
    let file = BufReader::new(File::open(file_path)?);
    sink.append(Decoder::new(file).unwrap());
    println!("Tocando arquivo: {:?}", file_path.file_name());
    Ok(())
}

fn get_user_input() -> String {
    println!("Digite 'd' próximo, 'a' anterior, 'w' tocar, 's' pausar, 'L' deletar ou 'q' sair: ");
    io::stdout().flush().expect("Falha ao limpar o buffer");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Falha ao ler a linha");
    input.trim().to_uppercase().to_string()
}

fn shuffle_files(mut files: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut rng = rand::thread_rng();
    files.shuffle(&mut rng);
    files
}
