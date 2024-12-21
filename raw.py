from copy import deepcopy
from Cryptodome.Cipher import AES

import time
import ddddocr
import threading
import requests
import base64
import json
import sys
import urllib3
from urllib3.exceptions import InsecureRequestWarning

if len(sys.argv) < 4:
    print('Usage: %s username password batchId <loop>' % sys.argv[0])
    exit(-1)

DEBUG_REQUEST_COUNT = 0
urllib3.disable_warnings(InsecureRequestWarning)

WorkThreadCount = 8
ocr = ddddocr.DdddOcr()

def pkcs7padding(data, block_size=16):
    if type(data) != bytearray and type(data) != bytes:
        raise TypeError("仅支持 bytearray/bytes 类型!")
    pl = block_size - (len(data) % block_size)
    return data + bytearray([pl for i in range(pl)])

class iCourses:
    mutex = threading.Lock()

    def __init__(self):
        self.aeskey = ''
        self.loginname = ''
        self.password = ''
        self.captcha = ''
        self.uuid = ''
        self.token = ''
        self.batchId = ''
        self.s = requests.session()
        
        self.is_login = False
        self.favorite = None
        self.select = None
        self.batchlist = None

        self.current = None
        self.error_code = 0
        self.try_if_capacity_full = True

    def safe_request(self, method, url, **kwargs):
        """永不放弃的请求包装器，持续重试直到成功"""
        while True:
            try:
                if method.lower() == 'get':
                    response = self.s.get(url, timeout=10, **kwargs)
                else:
                    response = self.s.post(url, timeout=10, **kwargs)
                return response
            except Exception as e:
                print(f"请求错误: {str(e)}, 正在重试...")
                time.sleep(0.5)
                continue

    def login(self, username, password):
        """无限重试的登录函数"""
        while True:
            try:
                index = 'https://icourses.jlu.edu.cn/'
                headers = {
                    'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9',
                    'Host': 'icourses.jlu.edu.cn',
                    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.5060.114 Safari/537.36 Edg/103.0.1264.62',
                }

                html = self.safe_request('get', index, headers=headers, verify=False).text
                start = html.find('"', html.find('loginVue.loginForm.aesKey')) + 1
                end = html.find('"', start)

                self.aeskey = html[start:end].encode('utf-8')
                self.loginname = username
                self.password = base64.b64encode(AES.new(self.aeskey, AES.MODE_ECB).encrypt(pkcs7padding(password.encode('utf-8'))))

                get_url = 'https://icourses.jlu.edu.cn/xsxk/auth/captcha'
                headers = {
                    'Host': 'icourses.jlu.edu.cn',
                    'Origin': 'https://icourses.jlu.edu.cn',
                    'Referer': 'https://icourses.jlu.edu.cn/xsxk/profile/index.html',
                    'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9',
                    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.5060.114 Safari/537.36 Edg/103.0.1264.62',
                }

                try:
                    data = json.loads(self.safe_request('post', get_url, headers=headers, verify=False).text)
                except:
                    print("验证码获取失败，重试中...")
                    time.sleep(0.5)
                    continue

                self.uuid = data['data']['uuid']
                captcha = data['data']['captcha']
                self.captcha = ocr.classification(base64.b64decode(captcha[captcha.find(',') + 1:]))

                login_url = 'https://icourses.jlu.edu.cn/xsxk/auth/login'
                payload = {
                    'loginname': self.loginname,
                    'password': self.password.decode('utf-8'),
                    'captcha': self.captcha,
                    'uuid': self.uuid
                }

                response = self.safe_request('post', login_url, headers=headers, data=payload, verify=False)
                response = json.loads(response.text)

                if response['code'] == 200 and response['msg'] == '登录成功':
                    self.token = response['data']['token']
                    s = ''
                    s += 'login success!\n'
                    s += '=' * 0x40 + '\n'
                    s += '\t\t@XH:   %s' % response['data']['student']['XH'] + '\n'
                    s += '\t\t@XM:   %s' % response['data']['student']['XM'] + '\n'
                    s += '\t\t@ZYMC: %s' % response['data']['student']['ZYMC'] + '\n'
                    s += '=' * 0x40 + '\n'
                    for c in response['data']['student']['electiveBatchList']:
                        s += '\t\t@name:      %s' % c['name'] + '\n'
                        s += '\t\t@BeginTime: %s' % c['beginTime'] + '\n'
                        s += '\t\t@EndTime:   %s' % c['endTime'] + '\n'
                        s += '=' * 0x40 + '\n'
                    print(s)
                    self.batchlist = response['data']['student']['electiveBatchList']
                    self.is_login = True
                    return True
                else:
                    print('login failed: %s' % response['msg'])
                    time.sleep(0.5)
                    continue

            except Exception as e:
                print(f"登录过程出错: {str(e)}, 重试中...")
                time.sleep(0.5)
                continue
    def setbatchId(self, idx):
        while True:
            try:
                url = 'https://icourses.jlu.edu.cn/xsxk/elective/user'
                headers = {
                    'Host': 'icourses.jlu.edu.cn',
                    'Origin': 'https://icourses.jlu.edu.cn',
                    'Referer': 'https://icourses.jlu.edu.cn/xsxk/profile/index.html',
                    'Authorization': self.token,
                    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.5060.114 Safari/537.36 Edg/103.0.1264.62',
                }

                try:
                    self.batchId = self.batchlist[idx]['code']
                except:
                    print('No such batch Id')
                    return

                payload = {
                    'batchId': self.batchId
                }

                response = json.loads(self.safe_request('post', url, headers=headers, data=payload, verify=False).text)
                if response['code'] != 200:
                    print("set batchid failed")
                    time.sleep(0.5)
                    continue

                c = self.batchlist[idx]
                print('Selected BatchId:')
                s = ''
                s += '=' * 0x40 + '\n'
                s += '\t\t@name:      %s' % c['name'] + '\n'
                s += '\t\t@BeginTime: %s' % c['beginTime'] + '\n'
                s += '\t\t@EndTime:   %s' % c['endTime'] + '\n'
                s += '=' * 0x40 + '\n'
                print(s)

                url = 'https://icourses.jlu.edu.cn/xsxk/elective/grablessons?batchId=' + self.batchId
                headers = {
                    'Host': 'icourses.jlu.edu.cn',
                    'Origin': 'https://icourses.jlu.edu.cn',
                    'Referer': 'https://icourses.jlu.edu.cn/xsxk/profile/index.html',
                    'Authorization': self.token,
                    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.5060.114 Safari/537.36 Edg/103.0.1264.62',
                }
                self.safe_request('get', url, headers=headers, verify=False)
                return

            except Exception as e:
                print(f"设置批次ID时出错: {str(e)}, 重试中...")
                time.sleep(0.5)
                continue

    def get_select(self):
        while True:
            try:
                post_url = 'https://icourses.jlu.edu.cn/xsxk/elective/select'
                headers = {
                    'Host': 'icourses.jlu.edu.cn',
                    'Origin': 'https://icourses.jlu.edu.cn',
                    'Referer': 'https://icourses.jlu.edu.cn/xsxk/elective/grablessons?batchId=' + self.batchId,
                    'Authorization': self.token,
                    'batchId': self.batchId,
                    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.5060.114 Safari/537.36 Edg/103.0.1264.62',
                }

                response = json.loads(self.safe_request('post', post_url, headers=headers, verify=False).text)
                if response['code'] == 200:
                    self.select = response['data']
                    return
                else:
                    print('get_select failed: %s' % response['msg'])
                    time.sleep(0.5)
                    continue

            except Exception as e:
                print(f"获取已选课程时出错: {str(e)}, 重试中...")
                time.sleep(0.5)
                continue

    def get_favorite(self):
        while True:
            try:
                post_url = 'https://icourses.jlu.edu.cn/xsxk/sc/clazz/list'
                headers = {
                    'Host': 'icourses.jlu.edu.cn',
                    'Origin': 'https://icourses.jlu.edu.cn',
                    'Referer': 'https://icourses.jlu.edu.cn/xsxk/elective/grablessons?batchId=' + self.batchId,
                    'Authorization': self.token,
                    'batchId': self.batchId,
                    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.5060.114 Safari/537.36 Edg/103.0.1264.62',
                }

                response = json.loads(self.safe_request('post', post_url, headers=headers, verify=False).text)
                if response['code'] == 200:
                    self.favorite = response['data']
                    return
                else:
                    print('get_favorite failed: %s' % response['msg'])
                    time.sleep(0.5)
                    continue

            except Exception as e:
                print(f"获取收藏课程时出错: {str(e)}, 重试中...")
                time.sleep(0.5)
                continue

    def select_favorite(self, ClassType, ClassId, SecretVal):
        while True:
            try:
                post_url = 'https://icourses.jlu.edu.cn/xsxk/sc/clazz/addxk'
                headers = {
                    'Host': 'icourses.jlu.edu.cn',
                    'Origin': 'https://icourses.jlu.edu.cn',
                    'Referer': 'https://icourses.jlu.edu.cn/xsxk/elective/grablessons?batchId=' + self.batchId,
                    'Authorization': self.token,
                    'batchId': self.batchId,
                    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.5060.114 Safari/537.36 Edg/103.0.1264.62',
                }

                payload = {
                    'clazzType': ClassType,
                    'clazzId': ClassId,
                    'secretVal': SecretVal
                }

                response = json.loads(self.safe_request('post', post_url, headers=headers, data=payload, verify=False).text)
                return response

            except Exception as e:
                print(f"选课过程出错: {str(e)}, 重试中...")
                time.sleep(0.5)
                continue

    def workThread(self, clazzType, clazzId, SecretVal, Name):
        tmp = deepcopy(self)
        
        global DEBUG_REQUEST_COUNT
        
        while True:
            try:
                response = tmp.select_favorite(clazzType, clazzId, SecretVal)
                
                code = response['code']
                msg = response['msg']

                self.mutex.acquire()
                try:
                    DEBUG_REQUEST_COUNT += 1

                    if self.current.get(clazzId) == 'doing':
                        if code == 200:
                            print('select [%s] success' % Name)
                            self.current[clazzId] = 'done'
                            self.mutex.release()
                            break
                        
                        elif code == 500:
                            if msg == '该课程已在选课结果中':
                                print('[%s] %s' % (Name, msg))
                                self.current[clazzId] = 'done'
                                self.mutex.release()
                                break

                            if msg == '本轮次选课暂未开始':
                                print('[%s]本轮次选课暂未开始' % (Name))
                                self.mutex.release()
                                continue

                            if msg == '课容量已满':
                                print(Name + "课容量已满")
                                self.mutex.release()
                                if self.try_if_capacity_full:
                                    continue
                                break

                            print('[%s] %s' % (Name, msg))
                            self.mutex.release()
                            continue

                        elif code == 401:
                            print(msg)
                            self.error_code = 401
                            self.mutex.release()
                            continue

                        else:
                            print('[%d]: failed, try again' % code)
                            self.mutex.release()
                            continue
                    else:
                        self.mutex.release()
                        break
                except:
                    if self.mutex.locked():
                        self.mutex.release()
                    raise

            except Exception as e:
                print(f"工作线程出错: {str(e)}, 重试中...")
                if self.mutex.locked():
                    self.mutex.release()
                time.sleep(0.5)
                continue

    def PrintSelect(self):
        print("=" * 20 + 'My Select' + '=' * 20)
        if self.select != None:
            for item in self.select:
                print('Teacher: %-10sName: %-20s Id: %-30s' % (item['SKJS'], item['KCM'], item['JXBID']))

    def PrintFavorite(self):
        print("=" * 20 + 'Favorite' + '=' * 20)
        if self.favorite != None:
            for item in self.favorite:
                print('Teacher: %-10sName: %-20s Id: %-30sclazzType: %-10s' % (
                    item['SKJS'], item['KCM'], item['JXBID'], item['teachingClassType']))
        print('=' * 48)

    def SelectMyFavorite(self):
        if self.favorite != None:
            for item in self.favorite:
                self.select_favorite(item['teachingClassType'], item['JXBID'], item['secretVal'])

    def FuckMyFavorite(self):
        while True:
            try:
                self.get_favorite()

                thread = {}
                if None != self.favorite:
                    self.current = {}

                    for item in self.favorite:
                        key = item['JXBID']
                        thread[key] = []

                        self.mutex.acquire()
                        self.current[key] = 'doing'
                        self.mutex.release()

                        args = (item['teachingClassType'], item['JXBID'], item['secretVal'], item['KCM'])

                        for i in range(WorkThreadCount):
                            thread[key].append(threading.Thread(target=self.workThread, args=args))
                            thread[key][-1].start()

                    for key in thread:
                        for t in thread[key]:
                            t.join()

                    print('本轮抢课结束，继续检查...')
                    return

            except Exception as e:
                print(f"抢课过程出错: {str(e)}, 重试中...")
                time.sleep(0.5)
                continue
if __name__ == '__main__':
    while True:
        try:
            a = iCourses()
            
            # 无限重试登录
            while not a.is_login:
                if a.login(sys.argv[1], sys.argv[2]):
                    break
                print('登录失败，重试中...')
                time.sleep(0.5)
            
            a.setbatchId(int(sys.argv[3]))
            a.get_favorite()
            a.PrintFavorite()
            a.FuckMyFavorite()

            if a.error_code == 401:
                print('会话过期，准备重新登录...')
                time.sleep(0.5)
                continue

            a.get_select()
            a.PrintSelect()
            print('DEBUG_REQUEST_COUNT: %d\n' % DEBUG_REQUEST_COUNT)

            if len(sys.argv) == 4:
                break
                
            time.sleep(0.5)
            
        except Exception as e:
            print(f"主循环出错: {str(e)}, 重试中...")
            time.sleep(0.5)
            continue

